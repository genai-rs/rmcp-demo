use anyhow::Result;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use dotenv::dotenv;
use opentelemetry::trace::{FutureExt, TraceContextExt};
use opentelemetry::{global, propagation::Extractor, trace::Tracer};
use serde_json::json;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::{error, info, instrument};

mod tracing_setup;
mod weather_service;

use crate::tracing_setup::init_tracing;
use crate::weather_service::WeatherService;

#[derive(Clone)]
struct AppState {
    weather_service: Arc<WeatherService>,
    tracer: Arc<global::BoxedTracer>,
}

struct HeaderMapExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderMapExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    init_tracing()?;

    let tracer = Arc::new(global::tracer("weather-assistant-rust"));
    let weather_service = Arc::new(WeatherService::new());

    let app_state = AppState {
        weather_service,
        tracer,
    };

    let app = Router::new()
        .route("/weather", post(handle_mcp_request))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8001")
        .await?;

    info!("Starting Rust Weather Assistant MCP Server on http://0.0.0.0:8001");
    info!("Streamable HTTP endpoint available at http://localhost:8001/weather");

    axum::serve(listener, app).await?;

    Ok(())
}

#[instrument(skip(state, headers))]
async fn handle_mcp_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let parent_context = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderMapExtractor(&headers))
    });

    let span = state.tracer.start_with_context("handle_mcp_request", &parent_context);
    let cx = parent_context.with_span(span);

    async move {
        match process_mcp_call(&state, payload).await {
            Ok(response) => (StatusCode::OK, Json(response)),
            Err(e) => {
                error!("Error processing MCP request: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to process request: {}", e)
                    })),
                )
            }
        }
    }
    .with_context(cx)
    .await
}

async fn process_mcp_call(state: &AppState, payload: serde_json::Value) -> Result<serde_json::Value> {
    if let Some(method) = payload.get("method").and_then(|m| m.as_str()) {
        match method {
            "tools/call" => {
                if let Some(name) = payload.get("params")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                {
                    let args = payload.get("params")
                        .and_then(|p| p.get("arguments"))
                        .cloned()
                        .unwrap_or_else(|| json!({}));

                    match name {
                        "get_weather" => {
                            let location = args.get("location")
                                .and_then(|l| l.as_str())
                                .unwrap_or("Unknown");

                            let weather = state.weather_service.get_weather(location).await?;
                            Ok(json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string(&weather)?
                                    }
                                ]
                            }))
                        }
                        "get_forecast" => {
                            let location = args.get("location")
                                .and_then(|l| l.as_str())
                                .unwrap_or("Unknown");
                            let days = args.get("days")
                                .and_then(|d| d.as_u64())
                                .unwrap_or(3) as usize;

                            let forecast = state.weather_service.get_forecast(location, days).await?;
                            Ok(json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string(&forecast)?
                                    }
                                ]
                            }))
                        }
                        _ => Ok(json!({
                            "error": format!("Unknown tool: {}", name)
                        }))
                    }
                } else {
                    Ok(json!({
                        "error": "Tool name not provided"
                    }))
                }
            }
            "tools/list" => {
                Ok(json!({
                    "tools": [
                        {
                            "name": "get_weather",
                            "description": "Get current weather for a specified location",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "location": {
                                        "type": "string",
                                        "description": "City name to get weather for"
                                    }
                                },
                                "required": ["location"]
                            }
                        },
                        {
                            "name": "get_forecast",
                            "description": "Get weather forecast for the specified location and number of days",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "location": {
                                        "type": "string",
                                        "description": "City name for forecast"
                                    },
                                    "days": {
                                        "type": "number",
                                        "description": "Number of days to forecast (1-7)",
                                        "default": 3
                                    }
                                },
                                "required": ["location"]
                            }
                        }
                    ]
                }))
            }
            _ => Ok(json!({
                "error": format!("Unknown method: {}", method)
            }))
        }
    } else {
        Ok(json!({
            "error": "Method not provided"
        }))
    }
}