use anyhow::Result;
use opentelemetry::{global, trace::TracerProvider, KeyValue};
use opentelemetry_langfuse::ExporterBuilder;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
    Resource,
};
use std::env;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_tracing() -> Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let service_name = env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "weather-assistant-rust".to_string());

    let resource = Resource::builder()
        .with_attributes(vec![
            KeyValue::new("service.name", service_name),
            KeyValue::new("service.version", "1.0.0"),
        ])
        .build();

    let langfuse_public_key = env::var("LANGFUSE_PUBLIC_KEY").ok();
    let langfuse_secret_key = env::var("LANGFUSE_SECRET_KEY").ok();
    let langfuse_host = env::var("LANGFUSE_HOST").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let mut builder = SdkTracerProvider::builder()
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource);

    if let (Some(public_key), Some(secret_key)) = (langfuse_public_key, langfuse_secret_key) {
        let exporter = ExporterBuilder::new()
            .with_host(&langfuse_host)
            .with_basic_auth(&public_key, &secret_key)
            .with_timeout(Duration::from_secs(10))
            .build()?;

        builder = builder.with_batch_exporter(exporter);
    }

    let provider = builder.build();
    let tracer = provider.tracer("weather-assistant-rust");

    global::set_tracer_provider(provider);

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let filter_layer = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(telemetry_layer)
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}