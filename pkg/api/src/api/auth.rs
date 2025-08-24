use crate::context::{Context, Invoker};
use rocket::{get, serde::json::Json};
use rocket_okapi::openapi;

#[openapi(tag = "Auth")]
#[get("/auth/me")]
pub fn get_me(ctx: Context) -> Json<Invoker> {
    Json(ctx.invoker.clone())
}
