use crate::api;
use crate::config::Config;
use crate::database::{self, DbPool};
use rocket::http::Status;
use rocket::response::{Body, Responder, Response};
use rocket::{Build, Rocket, State, get, serde::json::Json};
use rocket_okapi::swagger_ui::*;

#[get("/healthcheck")]
pub async fn healthcheck(db: &State<DbPool>) -> (Status, Json<HealthcheckResponse>) {
    let response = HealthcheckResponse {
        db: sqlx::query_scalar::<_, String>("select now()::varchar")
            .fetch_one(db.inner())
            .await
            .map_err(|e| println!("Database healthcheck failed: {}", e))
            .into(),
    };

    let ok = response.db == HealthcheckStatus::Healthy;

    if ok {
        (Status::Ok, Json(response))
    } else {
        (Status::ServiceUnavailable, Json(response))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum HealthcheckStatus {
    Healthy,
    Unhealthy,
}

impl<T, E> From<Result<T, E>> for HealthcheckStatus {
    fn from(result: Result<T, E>) -> Self {
        match result {
            Ok(_) => HealthcheckStatus::Healthy,
            Err(_) => HealthcheckStatus::Unhealthy,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthcheckResponse {
    pub db: HealthcheckStatus,
}

pub async fn build_server(config: Config) -> anyhow::Result<Rocket<Build>> {
    let db_pool = database::create_pool(&config.database_url).await?;

    Ok(rocket::build()
        .manage(config)
        .manage(db_pool)
        .mount("/", rocket::routes![healthcheck])
        .mount("/api/v1", api::get_api_routes())
        .mount(
            "/api/docs",
            make_swagger_ui(&SwaggerUIConfig {
                url: "/api/v1/openapi.json".to_owned(),
                ..Default::default()
            }),
        ))
}
