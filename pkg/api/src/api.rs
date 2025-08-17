pub mod auth;
pub mod device;
pub mod trigger;

use rocket_okapi::openapi_get_routes;

pub fn get_api_routes() -> Vec<rocket::Route> {
    openapi_get_routes![auth::get_me, trigger::trigger_action, device::get_devices,]
}
