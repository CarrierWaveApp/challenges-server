use sqlx::PgPool;
use std::collections::HashSet;

use crate::db;
use crate::models::club::MembershipMonitor;

/// Parse callsigns from text based on the monitor's format.
pub fn parse_callsigns(text: &str, format: &str) -> Vec<String> {
    match format {
        "one_per_line" => text
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_uppercase())
                }
            })
            .collect(),
        // Default: "callsign_notes" — Ham2K PoLo format
        _ => text
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#')
            })
            .filter_map(|line| {
                line.trim()
                    .split_whitespace()
                    .next()
                    .map(|cs| cs.to_uppercase())
            })
            .collect(),
    }
}

/// Run a single monitor check: fetch URL, diff membership, add/remove.
/// Returns (added, removed, total) on success, or an error string.
pub async fn check_monitor(
    pool: &PgPool,
    monitor: &MembershipMonitor,
) -> Result<(usize, usize, usize), String> {
    // Fetch the URL
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let resp = client
        .get(&monitor.url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch URL: {e}"))?
        .error_for_status()
        .map_err(|e| format!("URL returned error: {e}"))?;

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    let fetched_callsigns: HashSet<String> = parse_callsigns(&body, &monitor.format)
        .into_iter()
        .collect();

    if fetched_callsigns.is_empty() {
        return Err("No callsigns found in response".to_string());
    }

    // Get current members
    let current = db::clubs::get_member_callsigns(pool, monitor.club_id)
        .await
        .map_err(|e| format!("DB error fetching members: {e}"))?;
    let current_set: HashSet<String> = current.into_iter().collect();

    // Add new members
    let to_add: Vec<String> = fetched_callsigns
        .difference(&current_set)
        .cloned()
        .collect();

    if !to_add.is_empty() {
        let member_tuples: Vec<(String, String)> = to_add
            .iter()
            .map(|cs| (cs.clone(), "member".to_string()))
            .collect();
        db::clubs::add_members(pool, monitor.club_id, &member_tuples)
            .await
            .map_err(|e| format!("DB error adding members: {e}"))?;
    }

    // Remove stale members (if enabled)
    let mut removed = 0;
    if monitor.remove_stale {
        let to_remove: Vec<String> = current_set
            .difference(&fetched_callsigns)
            .cloned()
            .collect();

        if !to_remove.is_empty() {
            removed = db::clubs::remove_members_batch(pool, monitor.club_id, &to_remove)
                .await
                .map_err(|e| format!("DB error removing members: {e}"))? as usize;
        }
    }

    Ok((to_add.len(), removed, fetched_callsigns.len()))
}

/// Background loop that polls due monitors on an interval.
pub async fn monitor_loop(pool: PgPool, poll_minutes: u64) {
    let interval = tokio::time::Duration::from_secs(poll_minutes * 60);

    loop {
        tokio::time::sleep(interval).await;

        let monitors = match db::clubs::get_due_monitors(&pool).await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Failed to fetch due monitors: {e}");
                continue;
            }
        };

        if monitors.is_empty() {
            continue;
        }

        tracing::info!(count = monitors.len(), "Processing due membership monitors");

        for monitor in &monitors {
            let label = monitor
                .label
                .as_deref()
                .unwrap_or(&monitor.url);

            match check_monitor(&pool, monitor).await {
                Ok((added, removed, total)) => {
                    tracing::info!(
                        monitor_id = %monitor.id,
                        label,
                        added,
                        removed,
                        total,
                        "Monitor check complete"
                    );
                    let _ = db::clubs::update_monitor_status(
                        &pool,
                        monitor.id,
                        "ok",
                        total as i32,
                    )
                    .await;
                }
                Err(e) => {
                    tracing::warn!(
                        monitor_id = %monitor.id,
                        label,
                        error = %e,
                        "Monitor check failed"
                    );
                    let status = format!("error: {e}");
                    let _ = db::clubs::update_monitor_status(
                        &pool,
                        monitor.id,
                        &status,
                        monitor.last_member_count.unwrap_or(0),
                    )
                    .await;
                }
            }
        }
    }
}
