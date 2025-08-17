use rocket::{Rocket, Build};
use rocket_okapi::swagger_ui::*;
use crate::api;
use crate::config::Config;

pub fn build_server() -> Rocket<Build> {
    let config = Config::try_from_env().expect("Failed to load configuration");
    
    rocket::build()
        .manage(config)
        .mount("/api/v1", api::get_api_routes())
        .mount(
            "/api/docs",
            make_swagger_ui(&SwaggerUIConfig {
                url: "/api/v1/openapi.json".to_owned(),
                ..Default::default()
            }),
        )
}

pub async fn launch_server() -> Result<Rocket<rocket::Ignite>, rocket::Error> {
    build_server().launch().await
}