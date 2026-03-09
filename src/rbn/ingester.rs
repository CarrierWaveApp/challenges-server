use chrono::{NaiveTime, Utc};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use super::store::{freq_to_band, RbnSpot, SpotStore};

const RBN_HOST: &str = "telnet.reversebeacon.net";
const RBN_PORT: u16 = 7000;
const LOGIN_TIMEOUT_SECS: u64 = 10;
const BATCH_SIZE: usize = 100;
const BATCH_FLUSH_MS: u64 = 500;
const MAX_BACKOFF_SECS: u64 = 60;

/// Spawn the RBN telnet ingester as a background tokio task.
pub fn spawn_rbn_ingester(store: SpotStore, callsign: String) {
    tokio::spawn(async move {
        ingester_loop(store, callsign).await;
    });
}

async fn ingester_loop(store: SpotStore, callsign: String) {
    let mut backoff_secs = 1u64;

    loop {
        tracing::info!("RBN ingester: connecting to {}:{}", RBN_HOST, RBN_PORT);

        match run_connection(&store, &callsign).await {
            Ok(()) => {
                tracing::info!("RBN ingester: connection closed cleanly");
            }
            Err(e) => {
                tracing::error!("RBN ingester: connection error: {}", e);
            }
        }

        store.set_connected(false);
        tracing::info!("RBN ingester: reconnecting in {}s", backoff_secs);
        tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
        backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
    }
}

async fn run_connection(
    store: &SpotStore,
    callsign: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let stream = tokio::time::timeout(
        std::time::Duration::from_secs(LOGIN_TIMEOUT_SECS),
        TcpStream::connect((RBN_HOST, RBN_PORT)),
    )
    .await??;

    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    // Wait for login prompt — RBN sends "Please enter your call: " with no
    // trailing newline, so we must read raw bytes instead of lines.
    let login_deadline =
        tokio::time::Instant::now() + std::time::Duration::from_secs(LOGIN_TIMEOUT_SECS);

    let mut login_buf = [0u8; 256];
    let mut login_received = Vec::new();
    loop {
        let n = tokio::time::timeout_at(
            login_deadline,
            tokio::io::AsyncReadExt::read(&mut buf_reader, &mut login_buf),
        )
        .await??;

        if n == 0 {
            let received = String::from_utf8_lossy(&login_received);
            return Err(format!(
                "Connection closed before login prompt (received {} bytes: {:?})",
                login_received.len(),
                &received[..received.len().min(100)]
            ).into());
        }

        login_received.extend_from_slice(&login_buf[..n]);
        let received = String::from_utf8_lossy(&login_received);
        tracing::debug!("RBN ingester: login read {} bytes, total {}: {:?}", n, login_received.len(), &received[..received.len().min(80)]);

        if received.contains("call") || received.contains("login") || received.contains("callsign")
        {
            writer
                .write_all(format!("{}\n", callsign).as_bytes())
                .await?;
            tracing::info!("RBN ingester: logged in as {}", callsign);
            break;
        }
    }

    let mut lines = buf_reader.lines();

    store.set_connected(true);
    // Reset backoff on successful connection — caller handles backoff state,
    // but we signal success via the Ok return.

    let mut batch: Vec<RbnSpot> = Vec::with_capacity(BATCH_SIZE);
    let mut flush_deadline =
        tokio::time::Instant::now() + std::time::Duration::from_millis(BATCH_FLUSH_MS);

    loop {
        let result = tokio::time::timeout_at(flush_deadline, lines.next_line()).await;

        match result {
            Ok(Ok(Some(line))) => {
                if let Some(spot) = parse_spot_line(&line, store) {
                    batch.push(spot);
                }

                if batch.len() >= BATCH_SIZE {
                    store.push_batch(std::mem::take(&mut batch));
                    flush_deadline = tokio::time::Instant::now()
                        + std::time::Duration::from_millis(BATCH_FLUSH_MS);
                }
            }
            Ok(Ok(None)) => {
                // Connection closed
                if !batch.is_empty() {
                    store.push_batch(batch);
                }
                return Ok(());
            }
            Ok(Err(e)) => {
                if !batch.is_empty() {
                    store.push_batch(batch);
                }
                return Err(e.into());
            }
            Err(_) => {
                // Flush timeout — push whatever we have
                if !batch.is_empty() {
                    store.push_batch(std::mem::take(&mut batch));
                }
                flush_deadline = tokio::time::Instant::now()
                    + std::time::Duration::from_millis(BATCH_FLUSH_MS);
            }
        }
    }
}

/// Parse an RBN telnet spot line.
///
/// Format: `DX de KM3T-#:     14039.8  W1AW           CW    18 dB  25 WPM  CQ      1832Z`
fn parse_spot_line(line: &str, store: &SpotStore) -> Option<RbnSpot> {
    let line = line.trim();

    // Must start with "DX de "
    if !line.starts_with("DX de ") {
        return None;
    }

    let rest = &line[6..]; // after "DX de "

    // Spotter ends with ":"
    let colon_pos = rest.find(':')?;
    let spotter = rest[..colon_pos].trim().to_string();

    let after_colon = rest[colon_pos + 1..].trim();

    // Split remaining fields by whitespace
    let fields: Vec<&str> = after_colon.split_whitespace().collect();
    if fields.len() < 5 {
        return None;
    }

    // Field 0: frequency
    let frequency: f64 = fields[0].parse().ok()?;

    // Field 1: callsign
    let callsign = fields[1].to_uppercase();

    // Field 2: mode
    let mode = fields[2].to_uppercase();

    // Find SNR: look for "NN dB" pattern
    let mut snr: i32 = 0;
    let mut wpm: Option<u16> = None;
    let mut spot_type: Option<&str> = None;
    let mut time_str: Option<&str> = None;

    let mut i = 3;
    while i < fields.len() {
        if i + 1 < fields.len() && fields[i + 1] == "dB" {
            snr = fields[i].parse().unwrap_or(0);
            i += 2;
        } else if i + 1 < fields.len() && fields[i + 1] == "WPM" {
            wpm = fields[i].parse().ok();
            i += 2;
        } else if fields[i].ends_with('Z') && fields[i].len() == 5 {
            time_str = Some(fields[i]);
            i += 1;
        } else if matches!(
            fields[i],
            "CQ" | "DX" | "BEACON" | "NCDXF" | "DE" | "DXPED"
        ) {
            spot_type = Some(fields[i]);
            i += 1;
        } else {
            i += 1;
        }
    }

    // Filter: only keep CQ and DX spot types
    match spot_type {
        Some("CQ") | Some("DX") => {}
        _ => return None,
    }

    // Derive band from frequency
    let band = freq_to_band(frequency)?;

    // Parse timestamp: HHMM from "1832Z" → use today's UTC date
    let timestamp = if let Some(ts) = time_str {
        let hhmm = &ts[..4];
        if let Ok(time) = NaiveTime::parse_from_str(hhmm, "%H%M") {
            Utc::now()
                .date_naive()
                .and_time(time)
                .and_utc()
        } else {
            Utc::now()
        }
    } else {
        Utc::now()
    };

    Some(RbnSpot {
        id: store.next_id(),
        callsign,
        frequency,
        mode,
        snr,
        wpm,
        spotter,
        band,
        timestamp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cw_spot() {
        let store = SpotStore::new();
        let line = "DX de KM3T-#:     14039.8  W1AW           CW    18 dB  25 WPM  CQ      1832Z";
        let spot = parse_spot_line(line, &store).unwrap();
        assert_eq!(spot.callsign, "W1AW");
        assert_eq!(spot.spotter, "KM3T-#");
        assert!((spot.frequency - 14039.8).abs() < 0.01);
        assert_eq!(spot.mode, "CW");
        assert_eq!(spot.snr, 18);
        assert_eq!(spot.wpm, Some(25));
        assert_eq!(spot.band, "20m");
    }

    #[test]
    fn test_parse_ft8_spot() {
        let store = SpotStore::new();
        let line = "DX de W3LPL-#:     7074.0  N5XX           FT8    5 dB   CQ      2100Z";
        let spot = parse_spot_line(line, &store).unwrap();
        assert_eq!(spot.callsign, "N5XX");
        assert_eq!(spot.mode, "FT8");
        assert_eq!(spot.snr, 5);
        assert_eq!(spot.wpm, None);
        assert_eq!(spot.band, "40m");
    }

    #[test]
    fn test_parse_beacon_filtered() {
        let store = SpotStore::new();
        let line =
            "DX de KM3T-#:     14100.0  4U1UN          CW    30 dB  BEACON  1832Z";
        let spot = parse_spot_line(line, &store);
        assert!(spot.is_none(), "BEACON spots should be filtered out");
    }

    #[test]
    fn test_parse_non_spot_line() {
        let store = SpotStore::new();
        assert!(parse_spot_line("Please enter your callsign:", &store).is_none());
        assert!(parse_spot_line("", &store).is_none());
    }
}
