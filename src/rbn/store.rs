use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::Serialize;

/// Maximum spot age before eviction.
const MAX_AGE_SECS: i64 = 3600; // 1 hour

/// A single RBN spot parsed from the telnet stream.
#[derive(Debug, Clone, Serialize)]
pub struct RbnSpot {
    pub id: u64,
    pub callsign: String,
    pub frequency: f64,
    pub mode: String,
    pub snr: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wpm: Option<u16>,
    pub spotter: String,
    pub band: &'static str,
    pub timestamp: DateTime<Utc>,
}

/// In-memory store for RBN spots with automatic 1-hour eviction.
#[derive(Clone)]
pub struct SpotStore {
    inner: Arc<SpotStoreInner>,
}

struct SpotStoreInner {
    spots: RwLock<VecDeque<RbnSpot>>,
    next_id: AtomicU64,
    connected: RwLock<bool>,
}

impl SpotStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(SpotStoreInner {
                spots: RwLock::new(VecDeque::new()),
                next_id: AtomicU64::new(1),
                connected: RwLock::new(false),
            }),
        }
    }

    /// Allocate the next spot ID.
    pub fn next_id(&self) -> u64 {
        self.inner.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Push a batch of spots and evict stale entries.
    pub fn push_batch(&self, spots: Vec<RbnSpot>) {
        let mut store = self.inner.spots.write().unwrap();
        let cutoff = Utc::now() - chrono::Duration::seconds(MAX_AGE_SECS);

        // Evict stale spots from the front
        while let Some(front) = store.front() {
            if front.timestamp < cutoff {
                store.pop_front();
            } else {
                break;
            }
        }

        // Push new spots
        for spot in spots {
            store.push_back(spot);
        }
    }

    /// Set connection status.
    pub fn set_connected(&self, connected: bool) {
        *self.inner.connected.write().unwrap() = connected;
    }

    /// Get connection status.
    pub fn is_connected(&self) -> bool {
        *self.inner.connected.read().unwrap()
    }

    /// Query spots with filters. Returns (matching_total, limited_spots).
    pub fn query(&self, filter: &SpotFilter) -> (usize, Vec<RbnSpot>) {
        let store = self.inner.spots.read().unwrap();
        let cutoff = filter
            .since
            .unwrap_or_else(|| Utc::now() - chrono::Duration::seconds(MAX_AGE_SECS));

        let matching: Vec<RbnSpot> = store
            .iter()
            .rev() // newest first
            .filter(|s| s.timestamp >= cutoff)
            .cloned()
            .filter(|s| {
                if let Some(ref call) = filter.call {
                    s.callsign.eq_ignore_ascii_case(call)
                } else {
                    true
                }
            })
            .filter(|s| {
                if let Some(ref spotter) = filter.spotter {
                    s.spotter.eq_ignore_ascii_case(spotter)
                } else {
                    true
                }
            })
            .filter(|s| {
                if let Some(ref modes) = filter.modes {
                    modes.iter().any(|m| m.eq_ignore_ascii_case(&s.mode))
                } else {
                    true
                }
            })
            .filter(|s| {
                if let Some(ref band) = filter.band {
                    s.band.eq_ignore_ascii_case(band)
                } else {
                    true
                }
            })
            .filter(|s| {
                if let Some(min) = filter.min_freq {
                    s.frequency >= min
                } else {
                    true
                }
            })
            .filter(|s| {
                if let Some(max) = filter.max_freq {
                    s.frequency <= max
                } else {
                    true
                }
            })
            .collect();

        let total = matching.len();
        let limit = filter.limit.unwrap_or(100).min(500) as usize;
        let limited = matching.into_iter().take(limit).collect();

        (total, limited)
    }

    /// Get aggregate stats over a time window.
    pub fn stats(&self, minutes: u32) -> StatsResult {
        let store = self.inner.spots.read().unwrap();
        let cutoff = Utc::now() - chrono::Duration::minutes(minutes as i64);

        let mut bands: HashMap<&str, u64> = HashMap::new();
        let mut modes: HashMap<String, u64> = HashMap::new();
        let mut total: u64 = 0;

        for spot in store.iter() {
            if spot.timestamp >= cutoff {
                total += 1;
                *bands.entry(spot.band).or_default() += 1;
                *modes.entry(spot.mode.clone()).or_default() += 1;
            }
        }

        let spots_per_minute = if minutes > 0 {
            total as f64 / minutes as f64
        } else {
            0.0
        };

        StatsResult {
            minutes,
            total_spots: total,
            spots_per_minute,
            bands: bands.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            modes,
        }
    }

    /// Get active skimmers over a time window.
    pub fn skimmers(&self, minutes: u32, limit: u32) -> SkimmersResult {
        let store = self.inner.spots.read().unwrap();
        let cutoff = Utc::now() - chrono::Duration::minutes(minutes as i64);

        let mut skimmer_data: HashMap<String, SkimmerAccum> = HashMap::new();

        for spot in store.iter() {
            if spot.timestamp >= cutoff {
                let entry = skimmer_data
                    .entry(spot.spotter.clone())
                    .or_insert_with(|| SkimmerAccum {
                        spot_count: 0,
                        last_spot: spot.timestamp,
                        bands: Vec::new(),
                    });
                entry.spot_count += 1;
                if spot.timestamp > entry.last_spot {
                    entry.last_spot = spot.timestamp;
                }
                if !entry.bands.contains(&spot.band) {
                    entry.bands.push(spot.band);
                }
            }
        }

        let total_count = skimmer_data.len();
        let mut skimmers: Vec<SkimmerInfo> = skimmer_data
            .into_iter()
            .map(|(callsign, acc)| SkimmerInfo {
                callsign,
                spot_count: acc.spot_count,
                last_spot: acc.last_spot,
                bands: acc.bands.iter().map(|b| b.to_string()).collect(),
            })
            .collect();

        skimmers.sort_by(|a, b| b.spot_count.cmp(&a.spot_count));
        skimmers.truncate(limit.min(500) as usize);

        SkimmersResult {
            minutes,
            count: total_count,
            skimmers,
        }
    }

    /// Store size and oldest spot timestamp (for health reporting).
    pub fn health_info(&self) -> (usize, Option<DateTime<Utc>>) {
        let store = self.inner.spots.read().unwrap();
        let oldest = store.front().map(|s| s.timestamp);
        (store.len(), oldest)
    }
}

/// Filter parameters for spot queries.
pub struct SpotFilter {
    pub call: Option<String>,
    pub spotter: Option<String>,
    pub modes: Option<Vec<String>>,
    pub band: Option<String>,
    pub min_freq: Option<f64>,
    pub max_freq: Option<f64>,
    pub since: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
}

struct SkimmerAccum {
    spot_count: u64,
    last_spot: DateTime<Utc>,
    bands: Vec<&'static str>,
}

#[derive(Serialize)]
pub struct StatsResult {
    pub minutes: u32,
    pub total_spots: u64,
    pub spots_per_minute: f64,
    pub bands: HashMap<String, u64>,
    pub modes: HashMap<String, u64>,
}

#[derive(Serialize)]
pub struct SkimmersResult {
    pub minutes: u32,
    pub count: usize,
    pub skimmers: Vec<SkimmerInfo>,
}

#[derive(Serialize)]
pub struct SkimmerInfo {
    pub callsign: String,
    pub spot_count: u64,
    pub last_spot: DateTime<Utc>,
    pub bands: Vec<String>,
}

/// Derive band from frequency in kHz.
pub fn freq_to_band(freq: f64) -> Option<&'static str> {
    match freq as u32 {
        1800..=2000 => Some("160m"),
        3500..=4000 => Some("80m"),
        5330..=5410 => Some("60m"),
        7000..=7300 => Some("40m"),
        10100..=10150 => Some("30m"),
        14000..=14350 => Some("20m"),
        18068..=18168 => Some("17m"),
        21000..=21450 => Some("15m"),
        24890..=24990 => Some("12m"),
        28000..=29700 => Some("10m"),
        50000..=54000 => Some("6m"),
        _ => None,
    }
}
