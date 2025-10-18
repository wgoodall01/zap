use crate::error::{ApiError, ApiResult};
use crate::hdontap::{HdontapService, StreamId};
use axum::extract::Query;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
pub struct StreamQuery {
    /// Stream ID or URL to fetch
    pub id: String,
}

/// Redirect to a stream M3U8 URL from hdontap service
#[utoipa::path(
    get,
    path = "/wolp/stream",
    tag = "Wolp",
    operation_id = "wolp:stream",
    params(StreamQuery),
    responses(
        (status = 302, description = "Redirect to stream URL"),
        (status = 400, description = "Invalid stream ID format"),
        (status = 404, description = "Stream not found")
    )
)]
pub async fn stream(Query(params): Query<StreamQuery>) -> ApiResult<Response> {
    // Parse the id as StreamId (accepts both numeric IDs and URLs)
    let stream_id = params.id.parse::<StreamId>().map_err(|e| {
        ApiError::bad_request(anyhow::anyhow!("Invalid stream ID or URL format: {}", e))
    })?;

    // Get the M3U8 URL using HdontapService
    let hdontap_service = HdontapService::new();
    let m3u8_url = hdontap_service
        .get_stream_url(&stream_id)
        .await
        .map_err(|e| {
            ApiError::not_found(anyhow::anyhow!("Stream not found or error retrieving stream: {}", e))
        })?;

    tracing::info!("Redirecting to stream: {}", m3u8_url);
    Ok(Redirect::to(&m3u8_url.to_string()).into_response())
}