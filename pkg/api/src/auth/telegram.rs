use crate::error::ApiError;
use anyhow::{anyhow, Context, Result};
use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use base64::Engine;
use hmac::{Hmac, Mac};
use init_data_rs::validate_third_party;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TgUser {
    pub user_id: u64,
    pub name: String,
    pub tg_username: String,
    pub photo_url: Option<String>,
}

/// Payload from the Telegram Login Widget, sent as base64-encoded JSON.
#[derive(Debug, Clone, Deserialize)]
struct TgLoginWidgetPayload {
    id: u64,
    first_name: String,
    last_name: Option<String>,
    username: Option<String>,
    photo_url: Option<String>,
    auth_date: i64,
    hash: String,
}

/// Maximum age of auth data before we reject it (2 weeks), in seconds.
/// Used for both Mini-App init data and Login Widget payloads.
const AUTH_MAX_AGE_SECS: i64 = 2 * 7 * 24 * 60 * 60;

/// Parse and validate a Telegram Mini-App init data token into a [TgUser].
fn parse_init_data_token(raw_init_data: &str, bot_id: i64) -> Result<TgUser> {
    let id = validate_third_party(
        raw_init_data,
        bot_id,
        Some(AUTH_MAX_AGE_SECS as u64),
    )
    .context("Failed to validate Telegram init data")?;

    let user = id.user.ok_or_else(|| anyhow!("User not found in init data"))?;

    let name = [Some(user.first_name), user.last_name]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned();

    let user_id: u64 = user
        .id
        .checked_abs()
        .ok_or_else(|| anyhow!("Invalid user ID (negative integer) in init data"))?
        .try_into()
        .unwrap();

    Ok(TgUser {
        user_id,
        name,
        tg_username: user.username.unwrap_or_default(),
        photo_url: user.photo_url,
    })
}

/// Parse and validate a base64-encoded Telegram Login Widget token into a [TgUser].
fn parse_login_widget_token(base64_token: &str, bot_token: &str) -> Result<TgUser> {
    let json_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_token)
        .context("Invalid base64 in tg_data_check token")?;

    let payload: TgLoginWidgetPayload =
        serde_json::from_slice(&json_bytes).context("Invalid JSON in tg_data_check token")?;

    parse_login_widget_payload(&payload, bot_token)
}

/// Verify and extract a [TgUser] from a Telegram Login Widget payload.
///
/// See: https://core.telegram.org/widgets/login#checking-authorization
fn parse_login_widget_payload(
    payload: &TgLoginWidgetPayload,
    bot_token: &str,
) -> Result<TgUser> {
    // Build the data-check-string: sort all fields alphabetically (excluding hash),
    // format as key=value, join with \n.
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("auth_date={}", payload.auth_date));
    parts.push(format!("first_name={}", payload.first_name));
    parts.push(format!("id={}", payload.id));
    if let Some(ref last_name) = payload.last_name {
        parts.push(format!("last_name={last_name}"));
    }
    if let Some(ref photo_url) = payload.photo_url {
        parts.push(format!("photo_url={photo_url}"));
    }
    if let Some(ref username) = payload.username {
        parts.push(format!("username={username}"));
    }
    parts.sort();
    let data_check_string = parts.join("\n");

    // secret_key = SHA256(bot_token)
    let secret_key = Sha256::digest(bot_token.as_bytes());

    // hmac = HMAC-SHA256(data_check_string, secret_key)
    let mut mac =
        Hmac::<Sha256>::new_from_slice(&secret_key).map_err(|e| anyhow!("HMAC init error: {e}"))?;
    mac.update(data_check_string.as_bytes());
    let result = hex::encode(mac.finalize().into_bytes());

    if result != payload.hash {
        return Err(anyhow!("Hash mismatch"));
    }

    // Check auth_date freshness
    let now = chrono::Utc::now().timestamp();
    let age_secs = now - payload.auth_date;
    if age_secs > AUTH_MAX_AGE_SECS {
        return Err(anyhow!("Auth data is too old ({age_secs} seconds)"));
    }

    let name = [Some(payload.first_name.clone()), payload.last_name.clone()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned();

    Ok(TgUser {
        user_id: payload.id,
        name,
        tg_username: payload.username.clone().unwrap_or_default(),
        photo_url: payload.photo_url.clone(),
    })
}

#[async_trait]
impl FromRequestParts<crate::http_server::AppState> for TgUser {
    type Rejection = ApiError;

    #[tracing::instrument(name = "TgUser::from_request_parts", skip_all)]
    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::http_server::AppState,
    ) -> std::result::Result<Self, Self::Rejection> {
        let config = &state.config;

        // Pull out the `Authorization` header
        let header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                ApiError::unauthorized(anyhow!("Missing Authorization header"))
            })?;

        // Strip the `Bearer ` prefix
        let bearer_token = header.strip_prefix("Bearer ").ok_or_else(|| {
            ApiError::unauthorized(anyhow!(
                "Invalid Authorization header format: no 'Bearer' prefix"
            ))
        })?;

        // Try `tg_init_data:` prefix (Telegram Mini-App)
        if let Some(raw_init_data) = bearer_token.strip_prefix("tg_init_data:") {
            let bot_id = config.tg_bot_id().map_err(|e| {
                ApiError::internal_server_error(anyhow!("Failed to derive bot ID: {}", e))
            })?;

            return parse_init_data_token(raw_init_data, bot_id)
                .map_err(ApiError::unauthorized);
        }

        // Try `tg_data_check:` prefix (Telegram Login Widget)
        if let Some(b64_payload) = bearer_token.strip_prefix("tg_data_check:") {
            return parse_login_widget_token(b64_payload, &config.tg_bot_token)
                .map_err(ApiError::unauthorized);
        }

        // Neither prefix matched
        Err(ApiError::unauthorized(anyhow!(
            "Invalid Authorization token: expected 'tg_init_data:' or 'tg_data_check:' prefix"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign_login_widget_payload(payload: &serde_json::Value, bot_token: &str) -> String {
        let secret_key = Sha256::digest(bot_token.as_bytes());
        let mut parts: Vec<String> = Vec::new();

        // Collect all fields except "hash"
        if let Some(obj) = payload.as_object() {
            for (key, value) in obj {
                if key == "hash" {
                    continue;
                }
                let val_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    other => other.to_string(),
                };
                parts.push(format!("{key}={val_str}"));
            }
        }
        parts.sort();
        let data_check_string = parts.join("\n");

        let mut mac = Hmac::<Sha256>::new_from_slice(&secret_key).unwrap();
        mac.update(data_check_string.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    #[test]
    fn test_verify_login_widget_valid() {
        let bot_token = "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11";
        let auth_date = chrono::Utc::now().timestamp();

        let mut payload_json = serde_json::json!({
            "id": 12345678,
            "first_name": "Test",
            "username": "testuser",
            "auth_date": auth_date,
        });

        let hash = sign_login_widget_payload(&payload_json, bot_token);
        payload_json["hash"] = serde_json::Value::String(hash);

        let payload: TgLoginWidgetPayload = serde_json::from_value(payload_json).unwrap();
        let user = parse_login_widget_payload(&payload, bot_token).unwrap();
        assert_eq!(user.user_id, 12345678);
        assert_eq!(user.name, "Test");
        assert_eq!(user.tg_username, "testuser");
    }

    #[test]
    fn test_verify_login_widget_invalid_hash() {
        let bot_token = "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11";
        let auth_date = chrono::Utc::now().timestamp();

        let payload_json = serde_json::json!({
            "id": 12345678,
            "first_name": "Test",
            "username": "testuser",
            "auth_date": auth_date,
            "hash": "0000000000000000000000000000000000000000000000000000000000000000",
        });

        let payload: TgLoginWidgetPayload = serde_json::from_value(payload_json).unwrap();
        let result = parse_login_widget_payload(&payload, bot_token);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Hash mismatch"));
    }

    #[test]
    fn test_verify_login_widget_expired() {
        let bot_token = "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11";
        // Set auth_date to 15 days ago (beyond the 2-week max)
        let auth_date = chrono::Utc::now().timestamp() - 15 * 86400;

        let mut payload_json = serde_json::json!({
            "id": 12345678,
            "first_name": "Test",
            "username": "testuser",
            "auth_date": auth_date,
        });

        let hash = sign_login_widget_payload(&payload_json, bot_token);
        payload_json["hash"] = serde_json::Value::String(hash);

        let payload: TgLoginWidgetPayload = serde_json::from_value(payload_json).unwrap();
        let result = parse_login_widget_payload(&payload, bot_token);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too old"));
    }

    #[test]
    fn test_verify_login_widget_with_optional_fields() {
        let bot_token = "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11";
        let auth_date = chrono::Utc::now().timestamp();

        let mut payload_json = serde_json::json!({
            "id": 12345678,
            "first_name": "Test",
            "last_name": "User",
            "username": "testuser",
            "photo_url": "https://t.me/i/userpic/320/photo.jpg",
            "auth_date": auth_date,
        });

        let hash = sign_login_widget_payload(&payload_json, bot_token);
        payload_json["hash"] = serde_json::Value::String(hash);

        let payload: TgLoginWidgetPayload = serde_json::from_value(payload_json).unwrap();
        let user = parse_login_widget_payload(&payload, bot_token).unwrap();
        assert_eq!(user.user_id, 12345678);
        assert_eq!(user.name, "Test User");
        assert_eq!(user.tg_username, "testuser");
        assert_eq!(
            user.photo_url.as_deref(),
            Some("https://t.me/i/userpic/320/photo.jpg")
        );
    }
}
