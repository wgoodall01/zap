use crate::activity::{ActivityService, UserActivityCount};
use crate::context::Context;
use anyhow::Result;
use rocket::{get, serde::json::Json};
use rocket_okapi::openapi;
use schemars::JsonSchema;

#[openapi(tag = "Activity", operation_id = "activity:leaderboard")]
#[get("/activity/leaderboard?<top_n>&<reachback_seconds>")]
pub async fn leaderboard(
    ctx: Context,
    top_n: Option<u32>,
    reachback_seconds: Option<u32>,
) -> Result<Json<Leaderboard>, rocket::http::Status> {
    let leaders = ActivityService::new()
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

    Ok(Json(Leaderboard { leaders }))
}

#[derive(serde::Serialize, JsonSchema)]
pub struct Leaderboard {
    leaders: Vec<UserActivityCount>,
}
