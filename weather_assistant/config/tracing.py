"""Configure OpenTelemetry and Langfuse integration."""

from __future__ import annotations

import logging
import os
from typing import Optional

from langfuse import Langfuse
from opentelemetry import propagate, trace
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.trace.propagation.tracecontext import TraceContextTextMapPropagator


_LANGFUSE_CLIENT: Optional[Langfuse] = None
_TRACING_INITIALIZED = False


def setup_tracing():
    """Configure OpenTelemetry with Langfuse integration."""

    global _LANGFUSE_CLIENT, _TRACING_INITIALIZED

    if _TRACING_INITIALIZED:
        return _LANGFUSE_CLIENT

    # Check if TracerProvider is already set by Langfuse SDK
    if trace.get_tracer_provider() is not trace.ProxyTracerProvider():
        # Tracing already initialized
        _TRACING_INITIALIZED = True
        return _LANGFUSE_CLIENT

    resource = Resource.create(
        {
            "service.name": os.getenv("OTEL_SERVICE_NAME", "weather-assistant"),
            "service.version": "1.0.0",
        }
    )

    provider = TracerProvider(resource=resource)
    trace.set_tracer_provider(provider)

    # Get Langfuse configuration
    public_key = os.getenv("LANGFUSE_PUBLIC_KEY")
    secret_key = os.getenv("LANGFUSE_SECRET_KEY")
    host = os.getenv("LANGFUSE_HOST", os.getenv("LANGFUSE_BASE_URL", "http://localhost:3000"))

    # Set up Langfuse client
    # The Langfuse SDK v3 automatically integrates with OpenTelemetry when initialized
    if public_key and secret_key:
        _LANGFUSE_CLIENT = Langfuse(
            public_key=public_key,
            secret_key=secret_key,
            host=host,
            debug=os.getenv("LANGFUSE_DEBUG", "false").lower() == "true"
        )
        logger = logging.getLogger(__name__)
        logger.info(f"Langfuse client initialized for {host}")
    else:
        _LANGFUSE_CLIENT = None

    # Set up W3C TraceContext propagator (standard format)
    propagate.set_global_textmap(TraceContextTextMapPropagator())

    _TRACING_INITIALIZED = True
    return _LANGFUSE_CLIENT
