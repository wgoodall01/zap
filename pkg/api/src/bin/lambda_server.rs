use api::{config::Config, http_server};
use lambda_http::{Error, run};
use serde::Deserialize;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing subscriber for Lambda logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    // Load config from AWS Secrets Manager
    let config = load_config_from_secrets().await?;

    let app = http_server::build_server(config)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    // Run the Axum app on Lambda using lambda_http
    run(app).await
}

async fn load_secret(arn: &str) -> Result<serde_json::Value, Error> {
    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = aws_sdk_secretsmanager::Client::new(&aws_config);

    let response = client
        .get_secret_value()
        .secret_id(arn)
        .send()
        .await
        .map_err(|e| Error::from(format!("Could not load secret from SecretsManager: {}", e)))?;

    let secret_string = response
        .secret_string()
        .ok_or_else(|| Error::from("Secret value is empty"))?;

    serde_json::from_str(secret_string)
        .map_err(|e| Error::from(format!("Failed to parse secret JSON: {}", e)))
}

async fn load_config_from_secrets() -> Result<Config, Error> {
    let api_secret_arn = std::env::var("API_SECRET_ARN")
        .map_err(|_| Error::from("API_SECRET_ARN environment variable not set"))?;

    let db_secret_arn = std::env::var("DB_SECRET_ARN")
        .map_err(|_| Error::from("DB_SECRET_ARN environment variable not set"))?;

    // Load both secrets
    let api_secrets = load_secret(&api_secret_arn).await?;
    let db_secret_value = load_secret(&db_secret_arn).await?;

    // Parse database secret with struct
    #[derive(Deserialize)]
    struct DbSecret {
        host: String,
        port: u16,
        username: String,
        password: String,
        dbname: String,
    }
    let db_secret: DbSecret = serde_json::from_value(db_secret_value)
        .map_err(|e| Error::from(format!("Failed to parse DB secret: {}", e)))?;

    // Build DATABASE_URL from database credentials
    let database_url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        db_secret.username, db_secret.password, db_secret.host, db_secret.port, db_secret.dbname
    );

    // Merge API secrets with DATABASE_URL
    let mut merged_config = api_secrets;
    merged_config["DATABASE_URL"] = serde_json::Value::String(database_url);

    Config::from_value(&merged_config).map_err(|e| Error::from(e.to_string()))
}
