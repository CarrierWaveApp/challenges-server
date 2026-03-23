//! Background aggregator that syncs CW club memberships from rbn.telegraphy.de.
//!
//! On each cycle, for every club in the hardcoded registry:
//! 1. Ensures the club exists in the database (creates if missing)
//! 2. Fetches the member file (one callsign per line)
//! 3. Diff-syncs members: adds new, removes departed (preserves admin roles)

use sqlx::PgPool;

use crate::db;

/// A CW club sourced from rbn.telegraphy.de.
struct ClubDef {
    /// Display name of the club.
    name: &'static str,
    /// Short abbreviation (used as description).
    abbreviation: &'static str,
    /// URL of the member list file.
    members_url: &'static str,
}

const BASE_URL: &str = "https://rbn.telegraphy.de/src/members";

/// All clubs listed on https://rbn.telegraphy.de/info.
const CLUBS: &[ClubDef] = &[
    ClubDef { name: "CW Operators' Club", abbreviation: "CWops", members_url: "cwopsmembers.txt" },
    ClubDef { name: "FISTS", abbreviation: "FISTS", members_url: "fistsmembers.txt" },
    ClubDef { name: "First Class Operators' Club", abbreviation: "FOC", members_url: "focmembers.txt" },
    ClubDef { name: "High Speed Club", abbreviation: "HSC", members_url: "hscmembers.txt" },
    ClubDef { name: "Very High Speed Club", abbreviation: "VHSC", members_url: "vhscmembers.txt" },
    ClubDef { name: "Super High Speed Club", abbreviation: "SHSC", members_url: "shscmembers.txt" },
    ClubDef { name: "Extremely High Speed Club", abbreviation: "EHSC", members_url: "ehscmembers.txt" },
    ClubDef { name: "Straight Key Century Club", abbreviation: "SKCC", members_url: "skccmembers.txt" },
    ClubDef { name: "Arbeitsgemeinschaft CW", abbreviation: "AGCW", members_url: "agcwmembers.txt" },
    ClubDef { name: "North American QRP CW Club", abbreviation: "NAQCC", members_url: "naqccmembers.txt" },
    ClubDef { name: "Bug Users Group", abbreviation: "BUG", members_url: "bugmembers.txt" },
    ClubDef { name: "Russian CW Club", abbreviation: "RCWC", members_url: "rcwcmembers.txt" },
    ClubDef { name: "The Less Involved Data Society", abbreviation: "LIDS", members_url: "lidsmembers.txt" },
    ClubDef { name: "Novice Rig Round-Up", abbreviation: "NRR", members_url: "nrrmembers.txt" },
    ClubDef { name: "QRP Amateur Radio Club International", abbreviation: "QRP ARCI", members_url: "qrparcimembers.txt" },
    ClubDef { name: "Grupo Juizforano de CW", abbreviation: "CWJF", members_url: "cwjfmembers.txt" },
    ClubDef { name: "Tortugas CW Club", abbreviation: "TORCW", members_url: "torcwmembers.txt" },
    ClubDef { name: "Second Class Operators' Club", abbreviation: "SOC", members_url: "socmembers.txt" },
    ClubDef { name: "Union Française des Télégraphistes", abbreviation: "UFT", members_url: "uftmembers.txt" },
    ClubDef { name: "Essex CW ARC", abbreviation: "ECWARC", members_url: "ecwarcmembers.txt" },
    ClubDef { name: "Long Island CW Club", abbreviation: "LICW", members_url: "licwmembers.txt" },
    ClubDef { name: "EA CW Club", abbreviation: "EACW", members_url: "eacwmembers.txt" },
    ClubDef { name: "Marinefunker", abbreviation: "MF", members_url: "mfmembers.txt" },
    ClubDef { name: "A1 Club", abbreviation: "A1C", members_url: "a1cmembers.txt" },
    ClubDef { name: "Netherlands Telegraphy Club", abbreviation: "NTC", members_url: "ntcmembers.txt" },
    ClubDef { name: "Maritime Operators Radiotelegraphy Service", abbreviation: "MORSE", members_url: "morsemembers.txt" },
    ClubDef { name: "Four State QRP Group", abbreviation: "4SQRP", members_url: "4sqrpmembers.txt" },
    ClubDef { name: "30m CW Activity Group", abbreviation: "30CW", members_url: "30cwmembers.txt" },
    ClubDef { name: "Polski Klub Radiotelegrafistów", abbreviation: "SPCWC", members_url: "spcwcmembers.txt" },
    ClubDef { name: "Helvetia Telegraphy Club", abbreviation: "HTC", members_url: "htcmembers.txt" },
    ClubDef { name: "International CW Club U-QRQ", abbreviation: "UQRQC", members_url: "uqrqcmembers.txt" },
    ClubDef { name: "Grupo Português de CW", abbreviation: "GPCW", members_url: "gpcwmembers.txt" },
    ClubDef { name: "Marconi Club dell'A.R.I. di Loano", abbreviation: "MCARI", members_url: "mcarimembers.txt" },
    ClubDef { name: "Swedish High Speed Club", abbreviation: "SMHSC", members_url: "smhscmembers.txt" },
    ClubDef { name: "Die Österreichische CW Group", abbreviation: "OECWG", members_url: "oecwgmembers.txt" },
    ClubDef { name: "Chicken Fat Operators Club", abbreviation: "CFO", members_url: "cfomembers.txt" },
    ClubDef { name: "Club Francophone des Télégraphistes", abbreviation: "CFT", members_url: "cftmembers.txt" },
    ClubDef { name: "First Class Operators' Club Nominees", abbreviation: "FOCN", members_url: "focnmembers.txt" },
    ClubDef { name: "QRQ Crew Club", abbreviation: "QRQ Crew", members_url: "qrqcrewmembers.txt" },
];

/// Parse callsigns from a member list file.
///
/// Format: one callsign per line, first line may be a header ("callsign"),
/// empty lines and lines starting with '#' are ignored.
fn parse_member_file(body: &str) -> Vec<String> {
    body.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter(|line| line.to_lowercase() != "callsign") // skip header
        .filter_map(|line| {
            line.split_whitespace()
                .next()
                .map(|cs| cs.to_uppercase())
        })
        .collect()
}

/// Sync a single club: ensure it exists, fetch members, diff-sync.
async fn sync_club(
    pool: &PgPool,
    client: &reqwest::Client,
    club: &ClubDef,
) -> Result<(), String> {
    // Find or create the club
    let db_club = match db::clubs::get_club_by_name(pool, club.name).await {
        Ok(Some(c)) => c,
        Ok(None) => {
            tracing::info!(club = club.name, "Creating new club");
            db::clubs::create_club(pool, club.name, None, Some(club.abbreviation))
                .await
                .map_err(|e| format!("Failed to create club {}: {}", club.name, e))?
        }
        Err(e) => return Err(format!("Failed to look up club {}: {}", club.name, e)),
    };

    // Fetch member list
    let url = format!("{}/{}", BASE_URL, club.members_url);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch {}: {}", url, e))?
        .error_for_status()
        .map_err(|e| format!("HTTP error for {}: {}", url, e))?;

    let body = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response from {}: {}", url, e))?;

    let callsigns = parse_member_file(&body);

    if callsigns.is_empty() {
        tracing::warn!(club = club.name, url = %url, "Member file returned no callsigns, skipping sync");
        return Ok(());
    }

    // Diff-sync members
    let (added, removed) = db::clubs::sync_members(pool, db_club.id, &callsigns)
        .await
        .map_err(|e| format!("Failed to sync members for {}: {}", club.name, e))?;

    if added > 0 || removed > 0 {
        tracing::info!(
            club = club.name,
            total = callsigns.len(),
            added,
            removed,
            "Club membership synced"
        );
    }

    Ok(())
}

/// Main poll loop — runs one full sync cycle, then sleeps for the configured interval.
pub async fn poll_loop(pool: PgPool, client: reqwest::Client, interval_hours: u64) {
    let interval = std::time::Duration::from_secs(interval_hours * 3600);

    // Run immediately on startup, then on interval
    loop {
        tracing::info!("Starting club membership sync cycle ({} clubs)", CLUBS.len());

        let mut success = 0usize;
        let mut errors = 0usize;

        for club in CLUBS {
            match sync_club(&pool, &client, club).await {
                Ok(()) => success += 1,
                Err(e) => {
                    tracing::error!(club = club.name, error = %e, "Club sync failed");
                    errors += 1;
                }
            }
        }

        tracing::info!(
            success,
            errors,
            "Club membership sync cycle complete, next in {}h",
            interval_hours
        );

        tokio::time::sleep(interval).await;
    }
}
