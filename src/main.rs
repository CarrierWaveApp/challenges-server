mod aggregators;
mod auth;
mod config;
mod db;
mod error;
mod extractors;
mod handlers;
mod models;

use std::net::SocketAddr;

use axum::{
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use sqlx::postgres::PgPoolOptions;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "challenges_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = Config::from_env().expect("Failed to load configuration");

    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    tracing::info!("Database connected and migrations complete");

    // Spawn spot aggregators and TTL cleanup
    if config.spots_enabled {
        aggregators::spawn_aggregators(pool.clone(), &config);
        tracing::info!("Spots system enabled");
    }

    // Build router
    let app = create_router(pool.clone(), config.clone());

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn create_router(pool: sqlx::PgPool, config: Config) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes
    let public_routes = Router::new()
        .route("/challenges", get(handlers::list_challenges))
        .route("/challenges/:id", get(handlers::get_challenge))
        .route("/challenges/:id/join", post(handlers::join_challenge))
        .route(
            "/challenges/:id/leaderboard",
            get(handlers::get_leaderboard),
        )
        .route("/badges/:id/image", get(handlers::get_badge_image))
        .route("/programs", get(handlers::list_programs))
        .route("/programs/:slug", get(handlers::get_program))
        .route("/spots", get(handlers::list_spots))
        .route("/health", get(handlers::health_check))
        .route("/users/search", get(handlers::search_users))
        .route("/register", post(handlers::register))
        .layer(middleware::from_fn_with_state(
            pool.clone(),
            auth::optional_auth,
        ));

    // Authenticated routes
    let auth_routes = Router::new()
        .route("/challenges/:id/progress", post(handlers::report_progress))
        .route("/challenges/:id/progress", get(handlers::get_progress))
        .route("/challenges/:id/leave", delete(handlers::leave_challenge))
        .route(
            "/challenges/:id/participants/:callsign",
            get(handlers::get_participation_status),
        )
        .route(
            "/participants/:callsign/challenges",
            get(handlers::list_challenges_for_callsign),
        )
        .route("/friends/invite-link", get(handlers::get_invite_link))
        .route("/friends/requests", post(handlers::create_friend_request))
        .route(
            "/friends/suggestions",
            post(handlers::get_friend_suggestions),
        )
        .route("/friends", get(handlers::list_friends))
        .route(
            "/friends/requests/pending",
            get(handlers::list_pending_requests),
        )
        .route(
            "/friends/requests/:id/accept",
            post(handlers::accept_friend_request),
        )
        .route(
            "/friends/requests/:id/decline",
            post(handlers::decline_friend_request),
        )
        .route("/friends/:id", delete(handlers::remove_friend))
        .route("/activities", post(handlers::report_activity))
        .route("/activities/:id", delete(handlers::delete_activity))
        .route("/spots", post(handlers::create_self_spot))
        .route("/spots/:id", delete(handlers::delete_own_spot))
        .route("/feed", get(handlers::get_feed))
        .route("/clubs", get(handlers::get_clubs))
        .route("/clubs/:id", get(handlers::get_club_details))
        .route("/account", delete(handlers::delete_account))
        .layer(Extension(config.clone()))
        .layer(middleware::from_fn_with_state(
            pool.clone(),
            auth::require_auth,
        ));

    // Admin routes
    let admin_routes = Router::new()
        .route("/admin/challenges", post(handlers::create_challenge))
        .route("/admin/challenges/:id", put(handlers::update_challenge))
        .route("/admin/challenges/:id", delete(handlers::delete_challenge))
        .route(
            "/admin/challenges/:id/badges",
            post(handlers::upload_badge).get(handlers::list_badges),
        )
        .route("/admin/badges/:id", delete(handlers::delete_badge))
        .route(
            "/admin/challenges/:id/invites",
            post(handlers::generate_invite).get(handlers::list_invites),
        )
        .route("/admin/invites/:token", delete(handlers::revoke_invite))
        .route(
            "/admin/programs",
            post(handlers::create_program).get(handlers::admin_list_programs),
        )
        .route(
            "/admin/programs/:slug",
            put(handlers::update_program)
                .get(handlers::admin_get_program)
                .delete(handlers::delete_program),
        )
        .route("/admin/spots/:id", delete(handlers::admin_delete_spot))
        .layer(middleware::from_fn_with_state(
            config.admin_token,
            auth::require_admin,
        ));

    // Merge all v1 routes with a JSON 404 fallback for unmatched API paths
    let v1_routes = public_routes
        .merge(auth_routes)
        .merge(admin_routes)
        .fallback(api_not_found);

    // Friend invite page (server-rendered HTML for links opened in browsers)
    let invite_route = Router::new().route("/invite/:token", get(handlers::invite_page));

    // Static file serving for SPA (fallback to index.html for client-side routing)
    let serve_dir = ServeDir::new("web/dist").fallback(ServeFile::new("web/dist/index.html"));

    Router::new()
        .nest("/v1", v1_routes)
        .merge(invite_route)
        .fallback_service(serve_dir)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(pool)
}

async fn api_not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": {
                "code": "NOT_FOUND",
                "message": "Endpoint not found"
            }
        })),
    )
}
