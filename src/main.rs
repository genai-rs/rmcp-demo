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
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
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

#[derive(Debug, Deserialize, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
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
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let parent_context = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderMapExtractor(&headers))
    });

    let span = state.tracer.start_with_context("handle_mcp_request", &parent_context);
    let cx = parent_context.with_span(span);

    async move {
        // Try to parse as JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_value(payload) {
            Ok(req) => req,
            Err(e) => {
                error!("Failed to parse JSON-RPC request: {}", e);
                return (
                    StatusCode::OK,
                    Json(json!({
                        "jsonrpc": "2.0",
                        "error": {
                            "code": -32700,
                            "message": "Parse error",
                            "data": format!("{}", e)
                        },
                        "id": null
                    }))
                );
            }
        };

        let response = process_json_rpc_request(&state, request).await;
        (StatusCode::OK, Json(serde_json::to_value(response).unwrap()))
    }
    .with_context(cx)
    .await
}

async fn process_json_rpc_request(state: &AppState, request: JsonRpcRequest) -> JsonRpcResponse {
    let result = match request.method.as_str() {
        "initialize" => {
            json!({
                "protocolVersion": "0.1.0",
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "weather-assistant-rust",
                    "version": "1.0.0"
                }
            })
        }
        "initialized" => {
            json!({})
        }
        "tools/list" => {
            json!({
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
            })
        }
        "tools/call" => {
            match handle_tool_call(state, request.params).await {
                Ok(result) => result,
                Err(e) => {
                    return JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32603,
                            message: format!("Internal error: {}", e),
                            data: None,
                        }),
                        id: request.id,
                    };
                }
            }
        }
        _ => {
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
                id: request.id,
            };
        }
    };

    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(result),
        error: None,
        id: request.id,
    }
}

async fn handle_tool_call(state: &AppState, params: Option<Value>) -> Result<Value> {
    let params = params.ok_or_else(|| anyhow::anyhow!("Missing params for tool call"))?;

    let name = params.get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;

    let args = params.get("arguments")
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
        _ => Err(anyhow::anyhow!("Unknown tool: {}", name))
    }
}