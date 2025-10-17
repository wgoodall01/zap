use crate::lafitness::{CheckIn, LaFitnessService};
use rocket::serde::json::Json;
use rocket::post;
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Request body for fetching LA Fitness check-ins
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCheckinsRequest {
    /// LA Fitness account username
    pub username: String,
    /// LA Fitness account password
    pub password: String,
}

/// Response containing check-in history
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetCheckinsResponse {
    /// List of check-ins, sorted most-recent-first
    pub checkins: Vec<CheckIn>,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ErrorResponse {
    /// Error message
    pub error: String,
}

/// Fetch LA Fitness check-in history
///
/// This endpoint logs into the LA Fitness website, scrapes the check-in history,
/// and returns a list of check-ins sorted most-recent-first.
///
/// This is an unauthenticated endpoint - the LA Fitness credentials are provided
/// in the request body.
#[openapi(tag = "LA Fitness")]
#[post("/lafitness/get_checkins", data = "<request>")]
pub async fn get_checkins(
    request: Json<GetCheckinsRequest>,
) -> Result<Json<GetCheckinsResponse>, rocket::http::Status> {
    // Login to LA Fitness
    let service = LaFitnessService::login(request.username.clone(), request.password.clone())
        .await
        .map_err(|e| {
            eprintln!("Failed to login to LA Fitness: {}", e);
            rocket::http::Status::Unauthorized
        })?;

    // Fetch check-ins
    let checkins = service.get_checkins().await.map_err(|e| {
        eprintln!("Failed to fetch check-ins: {}", e);
        rocket::http::Status::InternalServerError
    })?;

    Ok(Json(GetCheckinsResponse { checkins }))
}
