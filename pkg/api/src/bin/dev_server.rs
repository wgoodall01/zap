use api::{config, http_server};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Load variables from the .env file, if it exists.
    let _ = dotenvy::dotenv().map(|p| tracing::info!("Loaded env from {p:?}"));

    let config = config::Config::try_from_env().expect("Failed to load configuration");

    match http_server::build_server(config).await {
        Ok(app) => {
            let port = std::env::var("PORT").unwrap_or_else(|_| "8000".to_string());
            let addr = format!("0.0.0.0:{}", port);

            let listener = TcpListener::bind(&addr)
                .await
                .unwrap_or_else(|_| panic!("Failed to bind to {}", addr));

            tracing::info!("Server listening on http://{}", listener.local_addr().unwrap());

            match axum::serve(listener, app).await {
                Ok(_) => tracing::info!("Server shut down gracefully."),
                Err(err) => tracing::error!("Server error: {}", err),
            }
        }
        Err(err) => tracing::error!("Failed to build server: {:?}", err),
    }
}
