use crate::error::ApiError;
use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use init_data_rs::validate_third_party;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TgUser {
    pub user_id: u64,
    pub name: String,
    pub tg_username: String,
    pub photo_url: Option<String>,
}

#[async_trait]
impl FromRequestParts<crate::http_server::AppState> for TgUser
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::http_server::AppState,
    ) -> Result<Self, Self::Rejection> {
        // Get the app config from state
        let config = &state.config;

        // Pull out the `Authorization` header
        let header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                ApiError::unauthorized(anyhow::anyhow!("Missing Authorization header"))
            })?;

        // Strip the `Bearer ` prefix
        let bearer_token = header.strip_prefix("Bearer ").ok_or_else(|| {
            ApiError::unauthorized(anyhow::anyhow!(
                "Invalid Authorization header format: no 'Bearer' prefix"
            ))
        })?;

        // Strip the `tg_init_data:` prefix
        let raw_init_data = bearer_token.strip_prefix("tg_init_data:").ok_or_else(|| {
            ApiError::unauthorized(anyhow::anyhow!(
                "Invalid Authorization header format: no 'tg_init_data:' prefix"
            ))
        })?;

        // Extract the init data
        let id = validate_third_party(raw_init_data, config.tg_bot_id, None).map_err(|e| {
            ApiError::unauthorized(anyhow::anyhow!("Failed to validate Telegram init data: {}", e))
        })?;

        // Extract the user (must be supplied)
        let user = id.user.ok_or_else(|| {
            ApiError::unauthorized(anyhow::anyhow!("User not found in init data"))
        })?;

        // Format the full name
        let name = [Some(user.first_name), user.last_name]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_owned();

        // Assert user ID is positive
        // (negative is a group chat, which we don't support as an auth principal)
        let user_id = user.id.checked_abs().ok_or_else(|| {
            ApiError::unauthorized(anyhow::anyhow!(
                "Invalid user ID (negative integer) in init data"
            ))
        })?;

        // Build the TgUser
        Ok(TgUser {
            user_id: user_id.try_into().unwrap(),
            name,
            tg_username: user.username.unwrap_or_default(),
            photo_url: user.photo_url,
        })
    }
}
