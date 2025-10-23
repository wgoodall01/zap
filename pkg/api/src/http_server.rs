use crate::api;
use crate::config::Config;
use axum::extract::{MatchedPath, State};
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db_pool: sqlx::Pool<sqlx::Postgres>,
}

// Implement FromRef to extract Config from AppState
impl axum::extract::FromRef<AppState> for Config {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

// Implement FromRef to extract PgPool from AppState
impl axum::extract::FromRef<AppState> for sqlx::Pool<sqlx::Postgres> {
    fn from_ref(state: &AppState) -> Self {
        state.db_pool.clone()
    }
}

/// Healthcheck endpoint
#[utoipa::path(
    get,
    path = "/healthcheck",
    tag = "Health",
    responses(
        (status = 200, description = "System is healthy", body = HealthcheckResponse),
        (status = 503, description = "System is unhealthy", body = HealthcheckResponse)
    )
)]
pub async fn healthcheck(State(state): State<AppState>) -> (StatusCode, Json<HealthcheckResponse>) {
    let response = HealthcheckResponse {
        db: sqlx::query_scalar::<_, String>("select now()::varchar")
            .fetch_one(&state.db_pool)
            .await
            .map_err(|e| {
                tracing::error!("Database healthcheck failed: {}", e);
                e
            })
            .into(),
    };

    let ok = response.db == HealthcheckStatus::Healthy;

    if ok {
        (StatusCode::OK, Json(response))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(response))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct HealthcheckResponse {
    pub db: HealthcheckStatus,
}

/// OpenAPI documentation structure
#[derive(OpenApi)]
#[openapi(
    paths(
        healthcheck,
        api::warmup,
        api::auth::get_me,
        api::auth::get_user,
        api::device::get_devices,
        api::trigger::trigger_action,
        api::activity::leaderboard,
        api::wolp::stream,
        api::lafitness::get_checkins
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Warmup", description = "Lambda warm-up endpoints"),
        (name = "Auth", description = "Authentication and user management"),
        (name = "Zap", description = "Zap controls - trigger shocks, vibrations, and sounds"),
        (name = "Activity", description = "User activity and leaderboards"),
        (name = "Wolp", description = "Stream redirection"),
        (name = "LA Fitness", description = "LA Fitness check-in scraping")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Add security scheme to OpenAPI documentation
struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("Telegram raw_init_data (with signature)")
                        .description(Some("Telegram MiniApp init-data token"))
                        .build(),
                ),
            );
        }
    }
}

/// Build the Axum server with all routes and middleware
pub async fn build_server(config: Config) -> anyhow::Result<Router> {
    // Use connect_lazy to avoid blocking Lambda init on Aurora wake-up
    // Set min_connections=1 to optimistically establish a connection in the background
    // This wakes up Aurora without blocking the initial request
    let db_pool = sqlx::postgres::PgPoolOptions::new()
        .min_connections(1)
        .max_connections(4)
        .connect_lazy(&config.database_url)?;

    let app_state = AppState { config, db_pool };

    // Build the router with custom tracing layer
    let app = Router::new()
        .route("/healthcheck", get(healthcheck))
        .nest("/api/v1", api::create_api_router())
        .merge(SwaggerUi::new("/api/docs").url("/api/v1/openapi.json", ApiDoc::openapi()))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    // Extract the matched path (URL template) from the request
                    let matched_path = request
                        .extensions()
                        .get::<MatchedPath>()
                        .map(|mp| mp.as_str())
                        .unwrap_or(request.uri().path());

                    tracing::info_span!(
                        "request",
                        method = %request.method(),
                        name = %matched_path,
                        status_code = tracing::field::Empty,
                    )
                })
                .on_request(|_request: &Request<_>, _span: &tracing::Span| {
                    tracing::debug!("started processing request");
                })
                .on_response(
                    |response: &axum::http::Response<_>,
                     latency: std::time::Duration,
                     span: &tracing::Span| {
                        span.record("status_code", response.status().as_u16());
                        tracing::debug!(
                            latency_ms = latency.as_millis(),
                            "finished processing request"
                        );
                    },
                )
                .on_failure(
                    |error: ServerErrorsFailureClass,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        tracing::error!(
                            latency_ms = latency.as_millis(),
                            error = %error,
                            "request failed"
                        );
                    },
                ),
        )
        .with_state(app_state);

    Ok(app)
}
