use axum::extract::Request;
use opentelemetry::Context;
use std::task::{Context as TaskContext, Poll};
use tower::{layer::Layer, Service};
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Stores the extracted OpenTelemetry context inside request extensions.
#[derive(Clone, Debug)]
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
    S: Service<Request>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let parent_context = opentelemetry::global::get_text_map_propagator(|prop| {
            prop.extract(&opentelemetry_http::HeaderExtractor(req.headers()))
        });

        tracing::Span::current().set_parent(parent_context.clone());
        req.extensions_mut()
            .insert(TraceParentContext(parent_context));

        self.inner.call(req)
    }
}
