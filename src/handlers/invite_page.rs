use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use sqlx::PgPool;

use crate::db;

/// GET /invite/:token
/// Renders an HTML page for friend invite links opened in a browser.
/// Shows the inviter's callsign and a deep link to open in Carrier Wave.
pub async fn invite_page(
    State(pool): State<PgPool>,
    Path(token): Path<String>,
) -> Response {
    // Look up the invite and the inviter's callsign
    let page = match build_invite_page(&pool, &token).await {
        Ok(html) => html,
        Err(_) => render_invite_page(None, &token),
    };

    (StatusCode::OK, [(header::CONTENT_TYPE, "text/html")], page).into_response()
}

async fn build_invite_page(
    pool: &PgPool,
    token: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let invite = db::get_friend_invite(pool, token).await?;

    let callsign = match invite {
        Some(ref inv) if inv.used_at.is_none() && inv.expires_at > chrono::Utc::now() => {
            let user = db::get_user_by_id(pool, inv.user_id).await?;
            user.map(|u| u.callsign)
        }
        _ => None,
    };

    Ok(render_invite_page(callsign.as_deref(), token))
}

fn render_invite_page(callsign: Option<&str>, token: &str) -> String {
    let deep_link = format!("carrierwave://invite/{}", token);

    let (title, heading, description) = match callsign {
        Some(cs) => (
            format!("{} wants to be friends on Carrier Wave", cs),
            format!("{} wants to be friends!", cs),
            format!("Open this link in Carrier Wave to add {} as a friend.", cs),
        ),
        None => (
            "Friend invite on Carrier Wave".to_string(),
            "You've been invited!".to_string(),
            "Open this link in Carrier Wave to accept this friend invite.".to_string(),
        ),
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    <meta property="og:title" content="{title}">
    <meta property="og:description" content="{description}">
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
            background: #0f172a;
            color: #e2e8f0;
            display: flex;
            align-items: center;
            justify-content: center;
            min-height: 100vh;
            padding: 1rem;
        }}
        .card {{
            background: #1e293b;
            border-radius: 1rem;
            padding: 2.5rem 2rem;
            max-width: 400px;
            width: 100%;
            text-align: center;
        }}
        .icon {{
            font-size: 3rem;
            margin-bottom: 1rem;
        }}
        h1 {{
            font-size: 1.25rem;
            font-weight: 600;
            margin-bottom: 0.75rem;
            color: #f8fafc;
        }}
        p {{
            font-size: 0.95rem;
            line-height: 1.5;
            color: #94a3b8;
            margin-bottom: 1.5rem;
        }}
        .open-btn {{
            display: inline-block;
            background: #3b82f6;
            color: #fff;
            text-decoration: none;
            font-weight: 600;
            font-size: 1rem;
            padding: 0.75rem 1.5rem;
            border-radius: 0.5rem;
            transition: background 0.15s;
        }}
        .open-btn:hover {{
            background: #2563eb;
        }}
        .footer {{
            margin-top: 1.5rem;
            font-size: 0.8rem;
            color: #64748b;
        }}
    </style>
</head>
<body>
    <div class="card">
        <div class="icon">📡</div>
        <h1>{heading}</h1>
        <p>{description}</p>
        <a class="open-btn" href="{deep_link}">Open in Carrier Wave</a>
        <div class="footer">Carrier Wave &mdash; Ham Radio Challenges</div>
    </div>
</body>
</html>"#,
        title = title,
        description = description,
        heading = heading,
        deep_link = deep_link,
    )
}
