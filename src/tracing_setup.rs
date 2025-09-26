use anyhow::Context;
use anyhow::Result;
use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::{HasExportConfig, SpanExporter};
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    resource::Resource,
    runtime::TokioCurrentThread,
    trace::{
        span_processor_with_async_runtime::BatchSpanProcessor as AsyncBatchSpanProcessor,
        BatchConfigBuilder, SdkTracerProvider,
    },
};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use std::{env, time::Duration};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

/// Initialise tracing so that `tracing` spans (including Tokio runtime spans)
/// are forwarded to the configured OpenTelemetry exporter and to stdout.
pub fn init_tracing() -> Result<SdkTracerProvider> {
    // Ensure trace context propagation (e.g. W3C traceparent headers).
    global::set_text_map_propagator(TraceContextPropagator::new());

    let traces_endpoint = env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
        .or_else(|_| {
            env::var("OTEL_EXPORTER_OTLP_ENDPOINT").map(|endpoint| format!("{endpoint}/v1/traces"))
        })
        .unwrap_or_else(|_| "http://localhost:4318/v1/traces".to_string());

    let mut exporter_builder = SpanExporter::builder().with_http();
    exporter_builder.export_config().endpoint = Some(traces_endpoint);
    let exporter = exporter_builder
        .build()
        .context("failed to build OTLP trace exporter")?;

    let batch_config = BatchConfigBuilder::default()
        .with_max_queue_size(2048)
        .with_scheduled_delay(Duration::from_millis(200))
        .build();

    let span_processor = AsyncBatchSpanProcessor::builder(exporter, TokioCurrentThread)
        .with_batch_config(batch_config)
        .build();

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new(SERVICE_NAME, "weather-assistant-rust"),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        ])
        .build();

    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_span_processor(span_processor)
        .build();

    let tracer = provider.tracer("weather-assistant");

    // Install the provider as global so other crates use it
    global::set_tracer_provider(provider.clone());

    // Forward tracing events (including Tokio internal spans when enabled) to OTEL
    // and keep console logging with env-based filtering.
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,tokio=info"));

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
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
