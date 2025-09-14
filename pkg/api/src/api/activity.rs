use crate::activity::ActivityService;
use crate::auth::{User, UserService};
use crate::context::Context;
use anyhow::Result;
use futures::{stream, StreamExt};
use rocket::{get, serde::json::Json};
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::Serialize;
use std::collections::BTreeMap;

#[openapi(tag = "Activity", operation_id = "activity:leaderboard")]
#[get("/activity/leaderboard?<top_n>&<reachback_seconds>")]
pub async fn leaderboard(
    ctx: Context,
    top_n: Option<u32>,
    reachback_seconds: Option<u32>,
) -> Result<Json<Leaderboard>, rocket::http::Status> {
    // Check that reachback_seconds isn't larger than 7 days
    if let Some(r) = reachback_seconds {
        if r > 7 * 24 * 60 * 60 {
            eprintln!("reachback_seconds too large: {}", r);
            return Err(rocket::http::Status::BadRequest);
        }
    }

    // Check that top_n isn't larger than 100
    if let Some(n) = top_n {
        if n > 100 {
            eprintln!("top_n too large: {}", n);
            return Err(rocket::http::Status::BadRequest);
        }
    }

    // Get activity counts with user IDs
    let activity_counts = ActivityService::new()
        .count_by_user(
            &ctx,
            reachback_seconds
                .map(|r| chrono::TimeDelta::seconds(r.into()))
                .unwrap_or_else(|| chrono::TimeDelta::days(1)),
            top_n.unwrap_or(100),
        )
        .await
        .map_err(|e| {
            eprintln!("Failed to get activity leaderboard: {:?}", e);
            rocket::http::Status::InternalServerError
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
                        eprintln!("Failed to fetch user {}: {:?}", activity_count.user_id, e);
                        rocket::http::Status::InternalServerError
                    })?;

                Ok::<UserActivityWithDetails, rocket::http::Status>(UserActivityWithDetails {
                    user,
                    counts: activity_count.counts,
                    total_actions: activity_count.total_actions,
                })
            }
        })
        .buffer_unordered(8)
        .collect::<Vec<Result<UserActivityWithDetails, rocket::http::Status>>>()
        .await
        .into_iter()
        .collect::<Result<Vec<UserActivityWithDetails>, rocket::http::Status>>()?;

    Ok(Json(Leaderboard { leaders }))
}

/// API response struct for leaderboard with full user details.
#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserActivityWithDetails {
    pub user: User,
    pub counts: BTreeMap<crate::activity::ActivityType, u64>,
    pub total_actions: u64,
}

#[derive(serde::Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Leaderboard {
    leaders: Vec<UserActivityWithDetails>,
}
