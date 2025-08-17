use rocket::{post, serde::json::Json};
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Action {
    Shock,
    Beep,
    Vibrate,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TriggerRequest {
    pub device_id: String,
    pub action: Action,
    pub duration_ms: u64,
}

#[openapi(tag = "Devices")]
#[post("/trigger", data = "<request>")]
pub fn trigger_action(request: Json<TriggerRequest>) -> Json<&'static str> {
    println!(
        "Triggering {:?} on device {} for {}ms",
        request.action, request.device_id, request.duration_ms
    );
    Json("Action triggered successfully")
}