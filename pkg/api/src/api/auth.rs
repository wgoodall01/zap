use rocket::{get, serde::json::Json};
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user_id: u64,
    pub username: String,
    pub email: Option<String>,
}

#[openapi(tag = "Auth")]
#[get("/auth/me")]
pub fn get_me() -> Json<User> {
    Json(User {
        user_id: 1,
        username: "test_user".to_owned(),
        email: Some("test@example.com".to_owned()),
    })
}