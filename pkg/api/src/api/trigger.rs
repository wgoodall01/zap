use crate::activity::{Activity, ActivityService};
use crate::config::Config;
use crate::context::Context;
use crate::error::ApiResult;
use crate::openshock::{
    ActionDuration, ActionIntensity, ControlMsg, ControlType, OpenshockService,
};
use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub enum Action {
    Shock {
        intensity: ActionIntensity,
        duration: ActionDuration,
    },
    Beep {
        intensity: ActionIntensity,
        duration: ActionDuration,
    },
    Vibrate {
        intensity: ActionIntensity,
        duration: ActionDuration,
    },
    Stop,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TriggerRequest {
    /// The UUIDs of the shocker devices to control
    pub shocker_ids: Vec<Uuid>,
    /// The action to perform with its parameters
    pub action: Action,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TriggerResponse {
    pub success: bool,
    pub message: String,
}

/// Trigger an action on one or more OpenShock devices
#[utoipa::path(
    post,
    path = "/zap/trigger",
    tag = "Zap",
    operation_id = "zap:trigger",
    request_body = TriggerRequest,
    responses(
        (status = 200, description = "Action triggered", body = TriggerResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer" = []))
)]
pub async fn trigger_action(
    ctx: Context,
    State(config): State<Config>,
    Json(request): Json<TriggerRequest>,
) -> ApiResult<Json<TriggerResponse>> {
    let openshock_service = OpenshockService::from_config(&config);
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
            ActionIntensity::new(10).unwrap(), // Stop doesn't need intensity but OpenShock API requires it
            ActionDuration::new(300).unwrap(), // Stop doesn't need duration but OpenShock API requires it
        ),
    };

    // Create control messages for all shockers
    let control_msgs: Vec<ControlMsg> = request
        .shocker_ids
        .iter()
        .map(|&shocker_id| ControlMsg {
            id: shocker_id,
            control_type: control_type.clone(),
            intensity,
            duration,
            exclusive: Some(true),
        })
        .collect();

    // Send the control command
    if let Err(e) = openshock_service.control(&ctx, &control_msgs).await {
        tracing::error!("Failed to send control command: {}", e);
        return Ok(Json(TriggerResponse {
            success: false,
            message: format!("Failed to trigger action: {}", e),
        }));
    }

    // Log the activity
    if let Err(e) = activity_service.log(&ctx, &activity).await {
        tracing::warn!("Failed to log activity: {}", e);
        // Don't fail the request if logging fails
    }

    Ok(Json(TriggerResponse {
        success: true,
        message: format!(
            "Successfully triggered {:?} on {} shocker(s)",
            request.action,
            request.shocker_ids.len()
        ),
    }))
}
