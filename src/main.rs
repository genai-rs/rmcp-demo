use anyhow::Result;
use axum::Router;
use dotenv::dotenv;
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpService,
};
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tracing::info;

mod trace_store;
mod trace_utils;
mod tracing_middleware;
mod tracing_setup;
mod weather_tools;

use crate::tracing_setup::init_tracing;
use crate::weather_tools::WeatherService;
use tracing_middleware::TracePropagationLayer;

const BIND_ADDRESS: &str = "0.0.0.0:8001";

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Initialize tracing with OpenTelemetry
    let tracer_provider = init_tracing()?;

    info!(
        "Starting Rust Weather Assistant MCP Server on http://{}",
        BIND_ADDRESS
    );
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
        .layer(TracePropagationLayer)
        .layer(CorsLayer::permissive());

    // Start the server
    let listener = tokio::net::TcpListener::bind(BIND_ADDRESS).await?;

    let shutdown_signal = async {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("Shutting down server...");
        } else {
            tracing::warn!("Failed to listen for Ctrl+C; forcing shutdown");
        }
    };

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    // Ensure all spans are flushed before exiting
    let shutdown_timeout = Duration::from_secs(10);
    let tracer_provider_for_shutdown = tracer_provider.clone();
    let mut shutdown_handle =
        tokio::task::spawn_blocking(move || tracer_provider_for_shutdown.shutdown());

    tokio::select! {
        shutdown_result = &mut shutdown_handle => {
            match shutdown_result {
                Ok(Ok(())) => info!("Tracer provider shut down cleanly"),
                Ok(Err(error)) => tracing::warn!(
                    error = %error,
                    "Tracer provider reported an error during shutdown"
                ),
                Err(join_error) => tracing::warn!(
                    error = %join_error,
                    "Tracer provider shutdown task panicked"
                ),
            }
        }
        _ = tokio::time::sleep(shutdown_timeout) => {
            tracing::warn!(
                timeout = ?shutdown_timeout,
                "Timed out waiting for tracer provider shutdown; aborting task"
            );
            shutdown_handle.abort();
            let _ = shutdown_handle.await;
        }
    }

    Ok(())
}
