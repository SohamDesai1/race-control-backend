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
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    println!("Listening on port 3000");
    serve(listener, app).await?;
    Ok(())
}
