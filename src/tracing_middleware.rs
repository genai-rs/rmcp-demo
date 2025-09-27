use axum::extract::Request;
use axum::response::Response;
use opentelemetry::Context;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use tower::{layer::Layer, Service};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::trace_store;

/// Stores the extracted OpenTelemetry context inside request extensions.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct TraceParentContext(pub Context);

#[derive(Clone, Default)]
pub struct TracePropagationLayer;

impl<S> Layer<S> for TracePropagationLayer {
    type Service = TracePropagationMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TracePropagationMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct TracePropagationMiddleware<S> {
    inner: S,
}

impl<S> Service<Request> for TracePropagationMiddleware<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        // Debug: Log incoming headers
        tracing::debug!("Incoming headers: {:?}", req.headers());

        // Check for traceparent header specifically
        if let Some(traceparent) = req.headers().get("traceparent") {
            tracing::info!("Received traceparent header: {:?}", traceparent);
        } else {
            tracing::debug!("No traceparent header found");
        }

        // Extract trace context from headers
        let parent_context = opentelemetry::global::get_text_map_propagator(|prop| {
            prop.extract(&opentelemetry_http::HeaderExtractor(req.headers()))
        });

        // Set current span parent
        tracing::Span::current().set_parent(parent_context.clone());

        // Store in request extensions for immediate use
        req.extensions_mut()
            .insert(TraceParentContext(parent_context.clone()));

        // Clone what we need for the async block
        let mut inner = self.inner.clone();
        let parent_context_clone = parent_context.clone();

        Box::pin(async move {
            // Call the inner service
            let response = inner.call(req).await?;

            // If response has mcp-session-id header, store the trace context
            if let Some(session_id) = response.headers().get("mcp-session-id") {
                if let Ok(session_str) = session_id.to_str() {
                    trace_store::store_trace_context(session_str.to_string(), parent_context_clone)
                        .await;
                    tracing::info!("Stored trace context for session: {}", session_str);
                }
            }

            Ok(response)
        })
    }
}
