// src/config.rs
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub admin_token: String,
    pub port: u16,
    pub base_url: Option<String>,
    pub invite_base_url: String,
    pub invite_expiry_days: i64,
    pub spots_enabled: bool,
    pub pota_aggregator_enabled: bool,
    pub sota_aggregator_enabled: bool,
    pub pota_stats_aggregator_enabled: bool,
    pub pota_stats_concurrency: usize,
    pub pota_stats_batch_size: i64,
    pub pota_stats_cycle_hours: u64,
    pub park_boundaries_enabled: bool,
    pub park_boundaries_batch_size: i64,
    pub park_boundaries_cycle_hours: u64,
    pub park_boundaries_stale_days: i64,
    pub park_boundaries_concurrency: usize,
    pub polish_park_boundaries_enabled: bool,
    pub polish_park_boundaries_batch_size: i64,
    pub polish_park_boundaries_cycle_hours: u64,
    pub polish_park_boundaries_stale_days: i64,
    pub polish_park_boundaries_concurrency: usize,
    pub historic_trails_enabled: bool,
    pub historic_trails_batch_size: i64,
    pub historic_trails_cycle_hours: u64,
    pub historic_trails_stale_days: i64,
    pub historic_trails_concurrency: usize,
    pub rbn_proxy_enabled: bool,
    pub rbn_proxy_callsign: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url =
            env::var("DATABASE_URL").map_err(|_| ConfigError::Missing("DATABASE_URL"))?;

        let admin_token =
            env::var("ADMIN_TOKEN").map_err(|_| ConfigError::Missing("ADMIN_TOKEN"))?;

        let port = env::var("PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .map_err(|_| ConfigError::Invalid("PORT must be a number"))?;

        let base_url = env::var("BASE_URL").ok();

        let invite_base_url = env::var("INVITE_BASE_URL")
            .unwrap_or_else(|_| "https://activities.carrierwave.app".to_string());

        let invite_expiry_days = env::var("INVITE_EXPIRY_DAYS")
            .unwrap_or_else(|_| "7".to_string())
            .parse()
            .map_err(|_| ConfigError::Invalid("INVITE_EXPIRY_DAYS must be a number"))?;

        let spots_enabled = env::var("SPOTS_ENABLED")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);

        let pota_aggregator_enabled = env::var("POTA_AGGREGATOR_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let sota_aggregator_enabled = env::var("SOTA_AGGREGATOR_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let pota_stats_aggregator_enabled = env::var("POTA_STATS_AGGREGATOR_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let pota_stats_concurrency: usize = env::var("POTA_STATS_CONCURRENCY")
            .unwrap_or_else(|_| "3".to_string())
            .parse()
            .unwrap_or(3);

        let pota_stats_batch_size: i64 = env::var("POTA_STATS_BATCH_SIZE")
            .unwrap_or_else(|_| "50".to_string())
            .parse()
            .unwrap_or(50);

        let pota_stats_cycle_hours: u64 = env::var("POTA_STATS_CYCLE_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse()
            .unwrap_or(24);

        let park_boundaries_enabled = env::var("PARK_BOUNDARIES_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let park_boundaries_batch_size: i64 = env::var("PARK_BOUNDARIES_BATCH_SIZE")
            .unwrap_or_else(|_| "20".to_string())
            .parse()
            .unwrap_or(20);

        let park_boundaries_cycle_hours: u64 = env::var("PARK_BOUNDARIES_CYCLE_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse()
            .unwrap_or(24);

        let park_boundaries_stale_days: i64 = env::var("PARK_BOUNDARIES_STALE_DAYS")
            .unwrap_or_else(|_| "90".to_string())
            .parse()
            .unwrap_or(90);

        let park_boundaries_concurrency: usize = env::var("PARK_BOUNDARIES_CONCURRENCY")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5);

        let polish_park_boundaries_enabled = env::var("POLISH_PARK_BOUNDARIES_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let historic_trails_enabled = env::var("HISTORIC_TRAILS_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let polish_park_boundaries_batch_size: i64 =
            env::var("POLISH_PARK_BOUNDARIES_BATCH_SIZE")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .unwrap_or(20);

        let polish_park_boundaries_cycle_hours: u64 =
            env::var("POLISH_PARK_BOUNDARIES_CYCLE_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24);

        let polish_park_boundaries_stale_days: i64 =
            env::var("POLISH_PARK_BOUNDARIES_STALE_DAYS")
                .unwrap_or_else(|_| "90".to_string())
                .parse()
                .unwrap_or(90);

        let polish_park_boundaries_concurrency: usize =
            env::var("POLISH_PARK_BOUNDARIES_CONCURRENCY")
                .unwrap_or_else(|_| "3".to_string())
                .parse()
                .unwrap_or(3);

        let historic_trails_batch_size: i64 = env::var("HISTORIC_TRAILS_BATCH_SIZE")
            .unwrap_or_else(|_| "20".to_string())
            .parse()
            .unwrap_or(20);

        let historic_trails_cycle_hours: u64 = env::var("HISTORIC_TRAILS_CYCLE_HOURS")
            .unwrap_or_else(|_| "168".to_string())
            .parse()
            .unwrap_or(168);

        let historic_trails_stale_days: i64 = env::var("HISTORIC_TRAILS_STALE_DAYS")
            .unwrap_or_else(|_| "180".to_string())
            .parse()
            .unwrap_or(180);

        let historic_trails_concurrency: usize = env::var("HISTORIC_TRAILS_CONCURRENCY")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5);

        let rbn_proxy_enabled = env::var("RBN_PROXY_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let rbn_proxy_callsign =
            env::var("RBN_PROXY_CALLSIGN").unwrap_or_else(|_| "W6JSV".to_string());

        Ok(Self {
            database_url,
            admin_token,
            port,
            base_url,
            invite_base_url,
            invite_expiry_days,
            spots_enabled,
            pota_aggregator_enabled,
            sota_aggregator_enabled,
            pota_stats_aggregator_enabled,
            pota_stats_concurrency,
            pota_stats_batch_size,
            pota_stats_cycle_hours,
            park_boundaries_enabled,
            park_boundaries_batch_size,
            park_boundaries_cycle_hours,
            park_boundaries_stale_days,
            park_boundaries_concurrency,
            polish_park_boundaries_enabled,
            polish_park_boundaries_batch_size,
            polish_park_boundaries_cycle_hours,
            polish_park_boundaries_stale_days,
            polish_park_boundaries_concurrency,
            historic_trails_enabled,
            historic_trails_batch_size,
            historic_trails_cycle_hours,
            historic_trails_stale_days,
            historic_trails_concurrency,
            rbn_proxy_enabled,
            rbn_proxy_callsign,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    Missing(&'static str),
    #[error("Invalid configuration: {0}")]
    Invalid(&'static str),
}
