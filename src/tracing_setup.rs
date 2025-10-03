use anyhow::Result;
use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_langfuse::ExporterBuilder;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator, resource::Resource, trace::SdkTracerProvider,
};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use std::env;
use tracing::{Metadata, Subscriber};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan, time::UtcTime},
    layer::{Filter, SubscriberExt},
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

/// Filter to exclude rmcp library internal spans that don't have proper parent context
#[derive(Debug, Clone)]
struct RmcpSpanFilter;

impl<S> Filter<S> for RmcpSpanFilter
where
    S: Subscriber,
{
    fn enabled(
        &self,
        meta: &Metadata<'_>,
        _cx: &tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        // Filter out rmcp internal spans that don't propagate trace context properly
        let name = meta.name();
        let target = meta.target();

        // Exclude serve_inner and streamable_http_session spans from rmcp
        if target.starts_with("rmcp")
            && (name == "serve_inner" || name == "streamable_http_session")
        {
            return false;
        }

        true
    }
}

/// Initialise tracing so that `tracing` spans (including Tokio runtime spans)
/// are forwarded to the configured OpenTelemetry exporter and to stdout.
pub fn init_tracing() -> Result<SdkTracerProvider> {
    // Ensure trace context propagation (e.g. W3C traceparent headers).
    global::set_text_map_propagator(TraceContextPropagator::new());

    // Build the resource with service information
    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new(SERVICE_NAME, "weather-assistant-rust"),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        ])
        .build();

    // Create the Langfuse exporter from environment configuration
    // This automatically wires up credentials and endpoint via LANGFUSE_* vars
    let exporter = ExporterBuilder::from_env()?.build()?;

    // Build the tracer provider with batch processing
    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();

    let tracer = provider.tracer("weather-assistant");

    // Install the provider as global so other crates use it
    global::set_tracer_provider(provider.clone());

    // Forward tracing events (including Tokio internal spans when enabled) to OTEL
    // and keep console logging with env-based filtering.
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tokio=info"));

    // Apply the filter to the OpenTelemetry layer to exclude unwanted rmcp spans
    let otel_layer = tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_filter(RmcpSpanFilter);

    let fmt_layer = fmt::layer()
        .with_timer(UtcTime::rfc_3339())
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT | FmtSpan::CLOSE);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    Ok(provider)
}
