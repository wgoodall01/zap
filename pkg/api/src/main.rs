mod api;
mod http_server;

#[tokio::main]
async fn main() {
    match http_server::launch_server().await {
        Ok(_) => println!("Rocket shut down gracefully."),
        Err(err) => println!("Rocket had an error: {}", err),
    }
}
