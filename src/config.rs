use serde::{Deserialize, Serialize};
use serde_variant::to_variant_name;
use std::{fs, path::Path};
use thiserror::Error;
use tracing::info;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// The URI for connecting to the database. For example:
    /// * Postgres: `postgres://root:12341234@localhost:5432/myapp_development`
    /// * Sqlite: `sqlite://db.sqlite?mode=rwc`
    pub uri: String,
    pub max_connections: Option<u32>,
    pub connection_timeout_seconds: Option<u64>,
}

/// SMTP configuration for application use
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmtpConfig {
    /// SMTP host. for example: localhost, smtp.gmail.com etc.
    pub host: String,
    /// SMTP port/
    pub port: u16,
    /// Auth SMTP server
    pub auth: MailerAuthConfig,
    pub from_email: String,
    pub to_email: String,
    pub frontend_url: String,
}

/// Authentication details for the mailer
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MailerAuthConfig {
    /// User
    pub user: String,
    /// Password
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}
impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        to_variant_name(self).expect("only enum supported").fmt(f)
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LogFormat {
    #[default]
    Compact,
    Pretty,
    Json,
}

/// Logger configuration for application use
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct LoggerConfig {
    /// Enable log write to stdout
    pub enable: bool,

    /// Set the logger level.
    ///
    /// * options: `trace` | `debug` | `info` | `warn` | `error`
    pub level: LogLevel,

    /// Set the logger format.
    ///
    /// * options: `compact` | `pretty` | `json`
    pub format: LogFormat,

    /// Override our custom tracing filter.
    ///
    /// Set this to your own filter if you want to see traces from internal
    /// libraries. See more [here](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives)
    pub override_filter: Option<String>,
}

/// Sentry configuration for application use
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SentryConfig {
    pub dsn: String,
    pub traces_sample_rate: f32,
}

/// Server configuration for application use
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// The address on which the server should listen on for incoming
    /// connections.
    #[serde(default = "default_binding")]
    pub binding: String,
    /// The port on which the server should listen for incoming connections.
    pub port: i32,
    /// The webserver host
    pub host: String,
}

fn default_binding() -> String {
    "localhost".to_string()
}

impl ServerConfig {
    #[must_use]
    pub fn full_url(&self) -> String {
        format!("{}:{}", self.binding, self.port)
    }
}

/// Complete application settings that combines all configuration layers
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppSettings {
    pub logger: LoggerConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub mailer: Option<SmtpConfig>,
    pub sentry: Option<SentryConfig>,
}

impl AppSettings {
    pub fn new(config: &Path) -> Result<Self, ConfigError> {
        info!(selected_path =? config, "loading environment from");
        let content = fs::read_to_string(config)?;
        Ok(toml::from_str::<Self>(&content)?)
    }
}

impl std::fmt::Display for AppSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content = toml::to_string(self).unwrap_or_default();
        write!(f, "{content}")
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Failed to read configuration file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse configuration: {0}")]
    ParseError(#[from] toml::de::Error),
}
