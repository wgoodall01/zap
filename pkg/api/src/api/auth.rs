use crate::auth::User;
use rocket::{get, serde::json::Json};
use rocket_okapi::openapi;


#[openapi(tag = "Auth")]
#[get("/auth/me")]
pub fn get_me(user: User) -> Json<User> {
    Json(user)
}