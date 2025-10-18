use crate::error::{ApiError, ApiResult};
use crate::lafitness::{CheckIn, LaFitnessService};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request body for fetching LA Fitness check-ins
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetCheckinsRequest {
    /// LA Fitness account username
    pub username: String,
    /// LA Fitness account password
    pub password: String,
}

/// Response containing check-in history
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetCheckinsResponse {
    /// List of check-ins, sorted most-recent-first
    pub checkins: Vec<CheckIn>,
}

/// Fetch LA Fitness check-in history
///
/// This endpoint logs into the LA Fitness website, scrapes the check-in history,
/// and returns a list of check-ins sorted most-recent-first.
///
/// This is an unauthenticated endpoint - the LA Fitness credentials are provided
/// in the request body.
#[utoipa::path(
    post,
    path = "/lafitness/get_checkins",
    tag = "LA Fitness",
    request_body = GetCheckinsRequest,
    responses(
        (status = 200, description = "Check-in history", body = GetCheckinsResponse),
        (status = 401, description = "Invalid LA Fitness credentials"),
        (status = 500, description = "Failed to fetch check-ins")
    )
)]
pub async fn get_checkins(
    Json(request): Json<GetCheckinsRequest>,
) -> ApiResult<Json<GetCheckinsResponse>> {
    // Login to LA Fitness
    let service = LaFitnessService::login(request.username.clone(), request.password.clone())
        .await
        .map_err(|e| {
            ApiError::unauthorized(anyhow::anyhow!("Failed to login to LA Fitness: {}", e))
        })?;

    // Fetch check-ins
    let checkins = service.get_checkins().await.map_err(|e| {
        ApiError::internal_server_error(anyhow::anyhow!("Failed to fetch check-ins: {}", e))
    })?;

    Ok(Json(GetCheckinsResponse { checkins }))
}
