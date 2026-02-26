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
    pub rbn_aggregator_enabled: bool,
    pub sota_aggregator_enabled: bool,
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

        let rbn_aggregator_enabled = env::var("RBN_AGGREGATOR_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        let sota_aggregator_enabled = env::var("SOTA_AGGREGATOR_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse()
            .unwrap_or(false);

        Ok(Self {
            database_url,
            admin_token,
            port,
            base_url,
            invite_base_url,
            invite_expiry_days,
            spots_enabled,
            pota_aggregator_enabled,
            rbn_aggregator_enabled,
            sota_aggregator_enabled,
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
