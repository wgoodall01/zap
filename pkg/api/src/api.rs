use rocket::{get, State};
use rocket_okapi::openapi;
use rocket_okapi::openapi_get_routes;

pub mod activity;
pub mod auth;
pub mod device;
pub mod trigger;

pub fn get_api_routes() -> Vec<rocket::Route> {
    openapi_get_routes![
        warmup,
        auth::get_me,
        trigger::trigger_action,
        device::get_devices,
        activity::leaderboard,
    ]
}

// Warmup route.
//
// On Lambda, we need to warm up a couple things:
// - The Aurora database, which will take ~10s to wake up from cold start.
// - The Lambda, which is already warm by the time this code runs, but is still triggered
//   by the incoming HTTP request.
//
// The frontend will call this route when it loads, so that we start warming up all the stuff (hopefully) before the user tries to do anything.
//
// This is an unauthenticated route, because we want to be able to call it before the user
// needs to use the API to sign in.
#[openapi(tag = "Warmup")]
#[get("/warmup")]
pub async fn warmup(pool: &State<sqlx::Pool<sqlx::Postgres>>) -> Result<(), rocket::http::Status> {
    // Connect to the database and make sure it's awake.
    sqlx::query!("select now(); -- warmup")
        .fetch_one(pool.inner())
        .await
        .map_err(|e| {
            eprintln!("Error warming up database: {}", e);
            rocket::http::Status::InternalServerError
        })?;

    Ok(())
}
