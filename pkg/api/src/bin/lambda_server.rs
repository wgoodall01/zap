use api::{config::Config, http_server};
use lambda_runtime::Error;
use lambda_web::launch_rocket_on_lambda;
use rocket::tokio;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Load config from AWS Secrets Manager
    let config = load_config_from_secrets().await?;

    let rocket = http_server::build_server(config);
    
    // Always use lambda_web since this is the Lambda entrypoint
    launch_rocket_on_lambda(rocket).await.map_err(|e| Error::from(e.to_string()))
}

async fn load_config_from_secrets() -> Result<Config, Error> {
    let secret_arn = std::env::var("API_SECRET_ARN")
        .map_err(|_| Error::from("API_SECRET_ARN environment variable not set"))?;
    
    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = aws_sdk_secretsmanager::Client::new(&aws_config);
    
    let response = client
        .get_secret_value()
        .secret_id(&secret_arn)
        .send()
        .await
        .map_err(|e| Error::from(format!("Could not load secrets from SecretsManager: {}", e)))?;
    
    let secret_string = response.secret_string()
        .ok_or_else(|| Error::from("Secret value is empty"))?;
    
    let secrets: serde_json::Value = serde_json::from_str(secret_string)
        .map_err(|e| Error::from(format!("Failed to parse secrets JSON: {}", e)))?;
    
    Config::from_value(&secrets).map_err(|e| Error::from(e.to_string()))
}