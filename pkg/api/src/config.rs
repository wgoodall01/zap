use anyhow::{Context, Result};
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Config {
    #[serde_as(as = "DisplayFromStr")]
    pub tg_bot_id: i64,
    pub database_url: String,
    pub openshock_api_key: String,
}

impl Config {
    /// Read the config from the environment.
    pub fn try_from_env() -> Result<Config> {
        // Get all the env vars. Panics on invalid Unicode.
        let env: serde_json::Value = std::env::vars()
            .map(|(k, v)| (k, serde_json::Value::String(v)))
            .collect();

        // Try to parse the config.
        let config: Config =
            serde_json::from_value(env).context("Failed to parse configuration")?;

        Ok(config)
    }

    /// Parse config from a JSON value (useful for SecretsManager).
    pub fn from_value(val: &serde_json::Value) -> Result<Config> {
        serde_json::from_value(val.clone()).with_context(|| {
            format!(
                "Failed to parse configuration from JSON. Input was: {}",
                val
            )
        })
    }
}
