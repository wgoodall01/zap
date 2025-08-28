use crate::config::Config;
use crate::context::Context;
use crate::openshock::{DeviceWithShockers, OpenshockService};
use rocket::{get, serde::json::Json, State};
use rocket_okapi::openapi;

#[openapi(tag = "Devices")]
#[get("/device")]
pub async fn get_devices(ctx: Context, config: &State<Config>) -> Json<Vec<DeviceWithShockers>> {
    let openshock_service = OpenshockService::from_config(config);

    match openshock_service.list_shockers(&ctx).await {
        Ok(devices) => Json(devices),
        Err(e) => {
            eprintln!("Failed to fetch devices from OpenShock: {}", e);
            Json(vec![])
        }
    }
}
