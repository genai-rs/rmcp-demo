import asyncio
import sys

from opentelemetry import trace
from opentelemetry.trace import format_trace_id
from opentelemetry.propagate import inject

from weather_assistant.config.tracing import setup_tracing

setup_tracing()

async def call_weather(location: str = "San Francisco", forecast_days: int = 3):
    tracer = trace.get_tracer(__name__)
    with tracer.start_as_current_span("cli_weather_request") as span:
        carrier: dict[str, str] = {}
        inject(carrier)

        from fastmcp.client import StreamableHttpTransport
        from fastmcp import Client

        transport = StreamableHttpTransport(
            url="http://localhost:8001/weather", headers=carrier
        )
        async with Client(transport) as client:
            await client.call_tool("get_weather", {"location": location})
            await client.call_tool(
                "get_forecast", {"location": location, "days": forecast_days}
            )

        trace_id = span.get_span_context().trace_id
        print(f"CLIENT_TRACE_ID={format_trace_id(trace_id)}")


def main():
    location = sys.argv[1] if len(sys.argv) > 1 else "San Francisco"
    forecast_days = int(sys.argv[2]) if len(sys.argv) > 2 else 3
    asyncio.run(call_weather(location, forecast_days))


if __name__ == "__main__":
    main()
