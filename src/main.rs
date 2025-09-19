use anyhow::Result;
use axum::Router;
use dotenv::dotenv;
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use tower_http::cors::CorsLayer;
use tracing::info;

mod tracing_setup;
mod weather_tools;
mod weather_service;  // Keep for the old tracing stuff if needed

use crate::tracing_setup::init_tracing;
use crate::weather_tools::WeatherService;

const BIND_ADDRESS: &str = "0.0.0.0:8001";

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Initialize tracing with OpenTelemetry and Langfuse
    init_tracing()?;

    info!("Starting Rust Weather Assistant MCP Server on http://{}", BIND_ADDRESS);
    info!("MCP endpoint available at http://localhost:8001/weather");

    // Create the MCP service with HTTP transport
    let service = StreamableHttpService::new(
        || Ok(WeatherService::new()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Create the router with the MCP service at /weather endpoint
    let router = Router::new()
        .nest_service("/weather", service)
        .layer(CorsLayer::permissive());

    // Start the server
    let listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.unwrap();
            info!("Shutting down server...");
        })
        .await?;

    Ok(())
}