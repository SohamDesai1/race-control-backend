mod handlers;
mod models;
mod routes;
mod utils;
use crate::routes::make_app;
use axum::serve;
use std::error::Error;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    // Bind to a TCP listener
    let app = make_app().await?;
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("Invalid PORT");

    let addr = format!("0.0.0.0:{}", port);

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on {}", addr);
    serve(listener, app).await?;
    Ok(())
}
