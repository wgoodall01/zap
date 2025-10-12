use crate::api;
use crate::config::Config;
use rocket::http::Status;
use rocket::{get, serde::json::Json, Build, Rocket, State};
use rocket_okapi::swagger_ui::*;

#[get("/healthcheck")]
pub async fn healthcheck(
    db: &State<sqlx::Pool<sqlx::Postgres>>,
) -> (Status, Json<HealthcheckResponse>) {
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
    // Use connect_lazy to avoid blocking Lambda init on Aurora wake-up
    // Set min_connections=1 to optimistically establish a connection in the background
    // This wakes up Aurora without blocking the initial request
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .min_connections(1)
        .max_connections(4)
        .connect_lazy(&config.database_url)?;

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
