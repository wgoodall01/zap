use crate::config::Config;
use init_data_rs::validate_third_party;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::{Deserialize, Serialize};
use rocket::State;
use rocket_okapi::okapi::schemars::JsonSchema;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TgUser {
    pub user_id: u64,
    pub name: String,
    pub tg_username: String,
    pub photo_url: Option<String>,
}

impl TgUser {
    pub fn from_authorization_header(header: &str, bot_id: i64) -> Option<Self> {
        // Check the Telegram init-data header.
        if let Some(raw_init_data) = header.strip_prefix("Bearer ") {
            // Extract the init data
            let id = validate_third_party(raw_init_data, bot_id, None).ok()?;

            // Extract the user (must be supplied)
            let user = id.user?;

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
            let user_id = user.id.checked_abs()?;

            // Build the TgUser
            return Some(TgUser {
                user_id: user_id.try_into().unwrap(),
                name,
                tg_username: user.username.unwrap_or_default(),
                photo_url: user.photo_url,
            });
        }

        None
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for TgUser {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Get the app config
        let config = request
            .guard::<&State<Config>>()
            .await
            .expect("Config not found in Rocket state");

        // Pull out the `Authorization` header.
        let Some(header) = request.headers().get_one("Authorization") else {
            println!("Missing Authorization header");
            return Outcome::Error((Status::Unauthorized, ()));
        };

        // Check the Telegram init-data header.
        if let Some(raw_init_data) = header.strip_prefix("Bearer ") {
            // Extract the init data
            let id = match validate_third_party(raw_init_data, config.tg_bot_id, None) {
                Ok(tgu) => tgu,
                Err(e) => {
                    println!("Failed to validate Telegram init data: {e}");
                    return Outcome::Error((Status::Unauthorized, ()));
                }
            };

            // Extract the user (must be supplied)
            let Some(user) = id.user else {
                println!("User not found in init data");
                return Outcome::Error((Status::Unauthorized, ()));
            };

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
            let Some(user_id) = user.id.checked_abs() else {
                println!("Invalid user ID (negative integer) in init data");
                return Outcome::Error((Status::Unauthorized, ()));
            };

            // Build the TgUser
            return Outcome::Success(TgUser {
                user_id: user_id.try_into().unwrap(),
                name,
                tg_username: user.username.unwrap_or_default(),
                photo_url: user.photo_url,
            });
        }

        Outcome::Error((Status::Unauthorized, ()))
    }
}
