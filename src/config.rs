use std::time::Duration;

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Database configuration
    pub database: DbConfig,
    /// Authentication configuration
    pub auth: AuthConfig,
    /// Tracing configuration
    pub tracing: TracingConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuthConfig {
    /// OpenID issuer
    pub issuer_url: Url,
    /// OAuth client id
    pub client_id: String,
    /// OAuth client secret
    pub client_secret: String,
    /// OAuth redirect (callback) url
    pub redirect_url: Url,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DbConfig {
    /// Connection URL
    pub url: String,
    /// Maximum number of connections to keep in the connection pool
    #[serde(default = "default_max_db_connections")]
    pub max_connections: usize,
    /// Maximum idle duration for individual connections, in seconds
    #[serde(default = "default_db_idle_timeout", with = "humantime_serde")]
    pub idle_timeout: Duration,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TracingConfig {
    /// Enable tracing
    pub enabled: bool,
}

pub const fn default_max_db_connections() -> usize {
    crate::database::DEFAULT_MAX_CONNECTIONS
}

pub const fn default_db_idle_timeout() -> Duration {
    crate::database::DEFAULT_IDLE_TIMEOUT
}
