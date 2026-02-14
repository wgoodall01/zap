use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Config {
    pub tg_bot_token: String,
    pub database_url: String,
    pub openshock_api_key: String,
}

impl Config {
    /// Derive the Telegram bot ID from the bot token (the part before `:`).
    pub fn tg_bot_id(&self) -> Result<i64> {
        let id_str = self
            .tg_bot_token
            .split(':')
            .next()
            .ok_or_else(|| anyhow::anyhow!("Invalid TG_BOT_TOKEN format: missing ':'"))?;
        id_str
            .parse::<i64>()
            .context("Invalid TG_BOT_TOKEN format: bot ID is not a valid integer")
    }

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
