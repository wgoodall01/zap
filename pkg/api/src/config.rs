use anyhow::{Context, Result};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Config {
    #[serde_as(as = "DisplayFromStr")]
    pub tg_bot_id: i64,
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
}
