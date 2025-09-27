use opentelemetry::trace::TraceContextExt;
use rand::Rng;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    schemars,
    service::RequestContext,
    tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::trace_store;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetWeatherArgs {
    /// City name to get weather for
    pub location: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetForecastArgs {
    /// City name for forecast
    pub location: String,
    /// Number of days to forecast (1-7)
    #[serde(default = "default_days")]
    pub days: u32,
}

fn default_days() -> u32 {
    3
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Weather {
    pub location: String,
    pub temperature: i32,
    pub condition: String,
    pub humidity: i32,
    pub wind_speed: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Forecast {
    pub day: i32,
    pub high: i32,
    pub low: i32,
    pub condition: String,
    pub precipitation_chance: i32,
}

#[derive(Clone)]
pub struct WeatherService {
    tool_router: ToolRouter<WeatherService>,
    // We could add state here if needed, e.g., for caching
    _state: Arc<Mutex<()>>,
}

#[tool_router]
impl WeatherService {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            _state: Arc::new(Mutex::new(())),
        }
    }

    #[tool(description = "Get current weather for a specified location")]
    #[instrument(skip(self, _request_context, params), fields(location))]
    async fn get_weather(
        &self,
        _request_context: RequestContext<RoleServer>,
        params: Parameters<GetWeatherArgs>,
    ) -> Result<CallToolResult, McpError> {
        let Parameters(args) = params;

        // Try to get stored trace context
        let stored_context = trace_store::get_current_trace_context().await;

        // Attach the stored context if available
        if let Some(ctx) = stored_context {
            tracing::Span::current().set_parent(ctx);
        }

        // Log the current span info
        let otel_context = tracing::Span::current().context();
        let span = otel_context.span();
        let span_context = span.span_context();
        let trace_id = span_context.trace_id();

        info!(
            %trace_id,
            location = %args.location,
            span_id = %span_context.span_id(),
            is_sampled = span_context.is_sampled(),
            "Handling get_weather request"
        );

        tracing::Span::current().record("location", &tracing::field::display(&args.location));

        let mut rng = rand::thread_rng();
        let weather_conditions = ["Sunny", "Cloudy", "Rainy", "Partly Cloudy"];

        let weather = Weather {
            location: args.location.clone(),
            temperature: rng.gen_range(15..=30),
            condition: weather_conditions[rng.gen_range(0..weather_conditions.len())].to_string(),
            humidity: rng.gen_range(40..=80),
            wind_speed: rng.gen_range(5..=25),
        };

        debug!(?weather, "Generated weather response");
        Ok(CallToolResult::structured(json!(&weather)))
    }

    #[tool(description = "Get weather forecast for the specified location and number of days")]
    #[instrument(skip(self, _request_context, params), fields(location, days))]
    async fn get_forecast(
        &self,
        _request_context: RequestContext<RoleServer>,
        params: Parameters<GetForecastArgs>,
    ) -> Result<CallToolResult, McpError> {
        let Parameters(args) = params;

        // Try to get stored trace context
        let stored_context = trace_store::get_current_trace_context().await;

        // Attach the stored context if available
        if let Some(ctx) = stored_context {
            tracing::Span::current().set_parent(ctx);
        }

        // Log the current span info
        let otel_context = tracing::Span::current().context();
        let span = otel_context.span();
        let span_context = span.span_context();
        let trace_id = span_context.trace_id();

        info!(
            %trace_id,
            location = %args.location,
            requested_days = args.days,
            span_id = %span_context.span_id(),
            is_sampled = span_context.is_sampled(),
            "Handling get_forecast request"
        );

        tracing::Span::current().record("location", &tracing::field::display(&args.location));
        tracing::Span::current().record("days", &tracing::field::display(&args.days));

        let mut rng = rand::thread_rng();
        let conditions = ["Sunny", "Cloudy", "Rainy", "Stormy"];
        let days = args.days.min(7);
        info!(location = %args.location, requested_days = args.days, effective_days = days, "Generating forecast");

        let forecast: Vec<Forecast> = (1..=days)
            .map(|day| Forecast {
                day: day as i32,
                high: rng.gen_range(20..=35),
                low: rng.gen_range(10..=20),
                condition: conditions[rng.gen_range(0..conditions.len())].to_string(),
                precipitation_chance: rng.gen_range(0..=100),
            })
            .collect();

        debug!(
            forecast_len = forecast.len(),
            ?forecast,
            "Generated forecast response"
        );

        Ok(CallToolResult::structured(json!({ "items": forecast })))
    }
}

#[tool_handler]
impl ServerHandler for WeatherService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "weather-assistant-rust".to_string(),
                version: "1.0.0".to_string(),
                title: None,
                website_url: None,
                icons: None,
            },
            instructions: Some("This server provides weather tools. Tools: get_weather (get current weather for a location), get_forecast (get weather forecast for multiple days).".to_string()),
        }
    }
}
