use crate::api;
use crate::config::Config;
use rocket::{get, serde::json::Json, Build, Rocket};
use rocket_okapi::swagger_ui::*;

#[get("/healthcheck")]
pub fn healthcheck() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

pub fn build_server(config: Config) -> Rocket<Build> {
    rocket::build()
        .manage(config)
        .mount("/", rocket::routes![healthcheck])
        .mount("/api/v1", api::get_api_routes())
        .mount(
            "/api/docs",
            make_swagger_ui(&SwaggerUIConfig {
                url: "/api/v1/openapi.json".to_owned(),
                ..Default::default()
            }),
        )
}
