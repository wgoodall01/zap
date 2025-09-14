use crate::auth::{User, UserService};
use crate::context::{Context, Invoker};
use anyhow::Result;
use rocket::{get, serde::json::Json};
use rocket_okapi::openapi;
use uuid::Uuid;

#[openapi(tag = "Auth", operation_id = "auth:me")]
#[get("/auth/me")]
pub fn get_me(ctx: Context) -> Json<Invoker> {
    Json(ctx.invoker.clone())
}

#[openapi(tag = "Auth", operation_id = "auth:get_user")]
#[get("/user/<id>")]
pub async fn get_user(ctx: Context, id: Uuid) -> Result<Json<User>, rocket::http::Status> {
    let user_service = UserService::new();
    let user = user_service.get(&ctx, id).await.map_err(|e| {
        eprintln!("Failed to get user {}: {:?}", id, e);
        rocket::http::Status::NotFound
    })?;

    Ok(Json(user))
}
