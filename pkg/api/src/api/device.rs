use crate::config::Config;
use crate::context::Context;
use crate::openshock::{DeviceWithShockers, OpenshockService};
use axum::Json;
use axum::extract::State;

/// List all OpenShock devices and shockers for the authenticated user
#[utoipa::path(
    get,
    path = "/zap/device",
    tag = "Zap",
    operation_id = "zap:device:list",
    responses(
        (status = 200, description = "List of devices with shockers", body = Vec<DeviceWithShockers>),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer" = []))
)]
pub async fn get_devices(
    ctx: Context,
    State(config): State<Config>,
) -> Json<Vec<DeviceWithShockers>> {
    let openshock_service = OpenshockService::from_config(&config);

    match openshock_service.list_shockers(&ctx).await {
        Ok(devices) => Json(devices),
        Err(e) => {
            tracing::error!("Failed to fetch devices from OpenShock: {}", e);
            Json(vec![])
        }
    }
}
