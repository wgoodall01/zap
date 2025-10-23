use crate::activity::ActivityService;
use crate::auth::{User, UserService};
use crate::context::Context;
use crate::error::{ApiError, ApiResult};
use axum::Json;
use axum::extract::Query;
use futures::{StreamExt, stream};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::{IntoParams, ToSchema};

/// Query parameters for the leaderboard endpoint
#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardQuery {
    /// Maximum number of users to return (max: 100)
    #[param(maximum = 100)]
    pub top_n: Option<u32>,
    /// Time window in seconds to look back (max: 7 days)
    #[param(maximum = 604800)]
    pub reachback_seconds: Option<u32>,
}

/// Get the activity leaderboard for the most active users
#[utoipa::path(
    get,
    path = "/activity/leaderboard",
    tag = "Activity",
    operation_id = "activity:leaderboard",
    params(LeaderboardQuery),
    responses(
        (status = 200, description = "Activity leaderboard", body = Leaderboard),
        (status = 400, description = "Invalid parameters"),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer" = []))
)]
pub async fn leaderboard(
    ctx: Context,
    Query(params): Query<LeaderboardQuery>,
) -> ApiResult<Json<Leaderboard>> {
    // Check that reachback_seconds isn't larger than 7 days
    if let Some(r) = params.reachback_seconds
        && r > 7 * 24 * 60 * 60
    {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "reachback_seconds too large: {} (max: 7 days)",
            r
        )));
    }

    // Check that top_n isn't larger than 100
    if let Some(n) = params.top_n
        && n > 100
    {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "top_n too large: {} (max: 100)",
            n
        )));
    }

    // Get activity counts with user IDs
    let activity_counts = ActivityService::new()
        .count_by_user(
            &ctx,
            params
                .reachback_seconds
                .map(|r| chrono::TimeDelta::seconds(r.into()))
                .unwrap_or_else(|| chrono::TimeDelta::days(1)),
            params.top_n.unwrap_or(100),
        )
        .await
        .map_err(|e| {
            ApiError::internal_server_error(anyhow::anyhow!(
                "Failed to get activity leaderboard: {}",
                e
            ))
        })?;

    // Fetch users in parallel with a concurrency limit of 8
    let user_service = UserService::new();
    let leaders: Vec<UserActivityWithDetails> = stream::iter(activity_counts.into_iter())
        .map(|activity_count| {
            let user_service = &user_service;
            let ctx = &ctx;
            async move {
                let user = user_service
                    .get(ctx, activity_count.user_id)
                    .await
                    .map_err(|e| {
                        ApiError::internal_server_error(anyhow::anyhow!(
                            "Failed to fetch user {}: {}",
                            activity_count.user_id,
                            e
                        ))
                    })?;

                Ok::<UserActivityWithDetails, ApiError>(UserActivityWithDetails {
                    user,
                    counts: activity_count.counts,
                    total_actions: activity_count.total_actions,
                })
            }
        })
        .buffer_unordered(8)
        .collect::<Vec<Result<UserActivityWithDetails, ApiError>>>()
        .await
        .into_iter()
        .collect::<Result<Vec<UserActivityWithDetails>, ApiError>>()?;

    Ok(Json(Leaderboard { leaders }))
}

/// API response struct for leaderboard with full user details.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserActivityWithDetails {
    pub user: User,
    pub counts: BTreeMap<crate::activity::ActivityType, u64>,
    pub total_actions: u64,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Leaderboard {
    leaders: Vec<UserActivityWithDetails>,
}
