mod api;
mod auth;
mod config;
mod http_server;

#[tokio::main]
async fn main() {
    // Load variables from the .env file, if it exists.
    let _ = dotenvy::dotenv().map(|p| println!("Loaded env from {p:?}"));

    match http_server::launch_server().await {
        Ok(_) => println!("Rocket shut down gracefully."),
        Err(err) => println!("Rocket had an error: {}", err),
    }
}
