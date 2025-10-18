use crate::auth::{User, UserService};
use crate::context::{Context, Invoker};
use crate::error::{ApiError, ApiResult};
use axum::extract::Path;
use axum::Json;
use uuid::Uuid;

/// Get the current authenticated user's invoker information
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "Auth",
    operation_id = "auth:me",
    responses(
        (status = 200, description = "Current invoker", body = Invoker),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer" = []))
)]
pub async fn get_me(ctx: Context) -> Json<Invoker> {
    Json(ctx.invoker.clone())
}

/// Get a user by their UUID
#[utoipa::path(
    get,
    path = "/user/{id}",
    tag = "Auth",
    operation_id = "auth:get_user",
    params(
        ("id" = Uuid, Path, description = "User UUID")
    ),
    responses(
        (status = 200, description = "User found", body = User),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found")
    ),
    security(("bearer" = []))
)]
pub async fn get_user(ctx: Context, Path(id): Path<Uuid>) -> ApiResult<Json<User>> {
    let user_service = UserService::new();
    let user = user_service
        .get(&ctx, id)
        .await
        .map_err(|e| ApiError::not_found(anyhow::anyhow!("Failed to get user {}: {}", id, e)))?;

    Ok(Json(user))
}
