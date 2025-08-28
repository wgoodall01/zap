use crate::activity::{Activity, ActivityService};
use crate::config::Config;
use crate::context::Context;
use crate::openshock::{ControlMsg, ControlType, Duration, Intensity, OpenshockService};
use rocket::http::Status;
use rocket::{post, serde::json::Json, State};
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum Action {
    Shock {
        intensity: Intensity,
        duration: Duration,
    },
    Beep {
        intensity: Intensity,
        duration: Duration,
    },
    Vibrate {
        intensity: Intensity,
        duration: Duration,
    },
    Stop,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TriggerRequest {
    /// The UUID of the shocker device to control
    pub shocker_id: Uuid,
    /// The action to perform with its parameters
    pub action: Action,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TriggerResponse {
    pub success: bool,
    pub message: String,
}

#[openapi(tag = "Devices")]
#[post("/trigger", data = "<request>")]
pub async fn trigger_action(
    ctx: Context,
    config: &State<Config>,
    request: Json<TriggerRequest>,
) -> Result<Json<TriggerResponse>, Status> {
    let openshock_service = OpenshockService::from_config(config);
    let activity_service = ActivityService::new();

    // Convert the request action to OpenShock ControlType and Activity
    let (control_type, activity, intensity, duration) = match request.action {
        Action::Shock {
            intensity,
            duration,
        } => (
            ControlType::Shock,
            Activity::Shock {
                intensity,
                duration,
            },
            intensity,
            duration,
        ),
        Action::Vibrate {
            intensity,
            duration,
        } => (
            ControlType::Vibrate,
            Activity::Vibrate {
                intensity,
                duration,
            },
            intensity,
            duration,
        ),
        Action::Beep {
            intensity,
            duration,
        } => (
            ControlType::Sound,
            Activity::Beep {
                intensity,
                duration,
            },
            intensity,
            duration,
        ),
        Action::Stop => (
            ControlType::Stop,
            Activity::Stop,
            Intensity::new(10).unwrap(), // Stop doesn't need intensity but OpenShock API requires it
            Duration::new(300).unwrap(), // Stop doesn't need duration but OpenShock API requires it
        ),
    };

    // Create the control message
    let control_msg = ControlMsg {
        id: request.shocker_id,
        control_type,
        intensity,
        duration,
        exclusive: Some(true),
    };

    // Send the control command
    if let Err(e) = openshock_service.control(&ctx, &[control_msg]).await {
        eprintln!("Failed to send control command: {}", e);
        return Ok(Json(TriggerResponse {
            success: false,
            message: format!("Failed to trigger action: {}", e),
        }));
    }

    // Log the activity
    if let Err(e) = activity_service.log(&ctx, &activity).await {
        eprintln!("Failed to log activity: {}", e);
        // Don't fail the request if logging fails
    }

    Ok(Json(TriggerResponse {
        success: true,
        message: format!(
            "Successfully triggered {:?} on shocker {}",
            request.action, request.shocker_id
        ),
    }))
}
