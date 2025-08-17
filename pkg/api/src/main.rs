use rocket::{get, post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::{openapi, openapi_get_routes, swagger_ui::*};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct User {
    user_id: u64,
    username: String,
    email: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Device {
    device_id: String,
    name: String,
    connected: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TriggerRequest {
    device_id: String,
    action: Action,
    duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Action {
    Shock,
    Beep,
    Vibrate,
}

/// # Get current user
///
/// Returns info about the current user.
#[openapi(tag = "Auth")]
#[get("/auth/me")]
fn get_me() -> Json<User> {
    Json(User {
        user_id: 1,
        username: "test_user".to_owned(),
        email: Some("test@example.com".to_owned()),
    })
}

/// # Trigger device action
///
/// Triggers an action on a specified device.
#[openapi(tag = "Devices")]
#[post("/trigger", data = "<request>")]
fn trigger_action(request: Json<TriggerRequest>) -> Json<&'static str> {
    println!(
        "Triggering {:?} on device {} for {}ms",
        request.action, request.device_id, request.duration_ms
    );
    Json("Action triggered successfully")
}

/// # Get devices
///
/// Returns a list of connected devices.
#[openapi(tag = "Devices")]
#[get("/devices")]
fn get_devices() -> Json<Vec<Device>> {
    Json(vec![
        Device {
            device_id: "device_001".to_owned(),
            name: "Test Device 1".to_owned(),
            connected: true,
        },
        Device {
            device_id: "device_002".to_owned(),
            name: "Test Device 2".to_owned(),
            connected: false,
        },
    ])
}

#[rocket::main]
async fn main() {
    let launch_result = rocket::build()
        .mount(
            "/",
            openapi_get_routes![
                get_me,
                trigger_action,
                get_devices,
            ],
        )
        .mount(
            "/swagger-ui/",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .launch()
        .await;
    match launch_result {
        Ok(_) => println!("Rocket shut down gracefully."),
        Err(err) => println!("Rocket had an error: {}", err),
    };
}
