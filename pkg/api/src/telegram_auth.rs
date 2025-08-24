use crate::config::Config;
use init_data_rs::validate_third_party;
use rocket::State;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::okapi::Map;
use rocket_okapi::okapi::openapi3::{SecurityScheme, SecuritySchemeData};
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TgUser {
    pub user_id: u64,
    pub name: String,
    pub tg_username: String,
    pub photo_url: Option<String>,
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

impl<'r> OpenApiFromRequest<'r> for TgUser {
    fn from_request_input(
        _gen: &mut rocket_okapi::r#gen::OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<RequestHeaderInput> {
        let security_scheme = SecurityScheme {
            data: SecuritySchemeData::Http {
                scheme: "bearer".to_owned(),
                bearer_format: Some("Telegram raw_init_data (with signature)".to_owned()),
            },
            description: Some("Telegram MiniApp init-data token".to_owned()),
            extensions: Map::new(),
        };

        let mut security_req = Map::new();
        security_req.insert("bearer".to_owned(), vec![]);

        Ok(RequestHeaderInput::Security(
            "bearer".to_owned(),
            security_scheme,
            security_req,
        ))
    }
}
