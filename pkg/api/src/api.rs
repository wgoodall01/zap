use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};

pub mod activity;
pub mod auth;
pub mod device;
pub mod lafitness;
pub mod trigger;
pub mod wolp;

/// Create the API router with all v1 endpoints
pub fn create_api_router() -> Router<crate::http_server::AppState> {
    Router::new()
        .route("/warmup", get(warmup_handler))
        .route("/auth/me", get(auth::get_me))
        .route("/user/:id", get(auth::get_user))
        .route("/zap/device", get(device::get_devices))
        .route("/zap/trigger", post(trigger::trigger_action))
        .route("/activity/leaderboard", get(activity::leaderboard))
        .route("/wolp/stream", get(wolp::stream))
        .route("/lafitness/get_checkins", post(lafitness::get_checkins))
}

// Wrapper handler that extracts db_pool from AppState
async fn warmup_handler(
    State(state): State<crate::http_server::AppState>,
) -> Result<StatusCode, StatusCode> {
    warmup(State(state.db_pool)).await
}

/// Warmup route.
///
/// On Lambda, we need to warm up a couple things:
/// - The Aurora database, which will take ~10s to wake up from cold start.
/// - The Lambda, which is already warm by the time this code runs, but is still triggered
///   by the incoming HTTP request.
///
/// The frontend will call this route when it loads, so that we start warming up all the stuff (hopefully) before the user tries to do anything.
///
/// This is an unauthenticated route, because we want to be able to call it before the user
/// needs to use the API to sign in.
#[utoipa::path(
    get,
    path = "/warmup",
    tag = "Warmup",
    responses(
        (status = 200, description = "Database warmed up successfully"),
        (status = 500, description = "Failed to warm up database")
    )
)]
pub async fn warmup(
    State(pool): State<sqlx::Pool<sqlx::Postgres>>,
) -> Result<StatusCode, StatusCode> {
    // Connect to the database and make sure it's awake.
    sqlx::query!("select now(); -- warmup")
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            tracing::error!("Error warming up database: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(StatusCode::OK)
}
