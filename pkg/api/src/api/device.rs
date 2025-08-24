use crate::telegram_auth::TgUser;
use rocket::{get, serde::json::Json};
use rocket_okapi::okapi::schemars::JsonSchema;
use rocket_okapi::openapi;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub device_id: String,
    pub name: String,
    pub connected: bool,
}

#[openapi(tag = "Devices")]
#[get("/device")]
pub fn get_devices(_user: TgUser) -> Json<Vec<Device>> {
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
