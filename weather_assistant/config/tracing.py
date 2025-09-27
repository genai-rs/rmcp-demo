"""Configure OpenTelemetry, Langfuse, and optional Jaeger exports."""

from __future__ import annotations

import os
from typing import Optional

from langfuse import Langfuse
from opentelemetry import propagate, trace
from opentelemetry.exporter.jaeger.thrift import JaegerExporter
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.trace.propagation.tracecontext import TraceContextTextMapPropagator


_LANGFUSE_CLIENT: Optional[Langfuse] = None
_TRACING_INITIALIZED = False


def _build_jaeger_exporter() -> Optional[JaegerExporter]:
    """Create a Jaeger exporter when configuration is provided."""

    endpoint = os.getenv("JAEGER_ENDPOINT")
    if endpoint:
        return JaegerExporter(
            collector_endpoint=endpoint,
            username=os.getenv("JAEGER_USERNAME"),
            password=os.getenv("JAEGER_PASSWORD"),
        )

    agent_host = os.getenv("JAEGER_AGENT_HOST")
    if agent_host:
        agent_port = int(os.getenv("JAEGER_AGENT_PORT", "6831"))
        return JaegerExporter(agent_host_name=agent_host, agent_port=agent_port)

    return None


def setup_tracing():
    """Configure OpenTelemetry, Langfuse, and optional Jaeger export."""

    global _LANGFUSE_CLIENT, _TRACING_INITIALIZED

    if _TRACING_INITIALIZED:
        return _LANGFUSE_CLIENT

    resource = Resource.create(
        {
            "service.name": os.getenv("OTEL_SERVICE_NAME", "weather-assistant"),
            "service.version": "1.0.0",
        }
    )

    provider = TracerProvider(resource=resource)

    jaeger_exporter = _build_jaeger_exporter()
    if jaeger_exporter is not None:
        provider.add_span_processor(BatchSpanProcessor(jaeger_exporter))

    trace.set_tracer_provider(provider)

    # Set up W3C TraceContext propagator (standard format)
    propagate.set_global_textmap(TraceContextTextMapPropagator())

    public_key = os.getenv("LANGFUSE_PUBLIC_KEY")
    secret_key = os.getenv("LANGFUSE_SECRET_KEY")
    host = os.getenv("LANGFUSE_HOST", os.getenv("LANGFUSE_BASE_URL", "http://localhost:3000"))

    if public_key and secret_key:
        _LANGFUSE_CLIENT = Langfuse(public_key=public_key, secret_key=secret_key, host=host)
    else:
        _LANGFUSE_CLIENT = None

    _TRACING_INITIALIZED = True
    return _LANGFUSE_CLIENT
