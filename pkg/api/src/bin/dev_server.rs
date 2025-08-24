use api::{config, http_server};

#[tokio::main]
async fn main() {
    // Load variables from the .env file, if it exists.
    let _ = dotenvy::dotenv().map(|p| println!("Loaded env from {p:?}"));

    let config = config::Config::try_from_env().expect("Failed to load configuration");

    match http_server::build_server(config).await {
        Ok(rocket) => match rocket.launch().await {
            Ok(_) => println!("Rocket shut down gracefully."),
            Err(err) => println!("Rocket had an error: {}", err),
        },
        Err(err) => println!("Failed to build server: {:?}", err),
    }
}
