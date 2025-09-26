"""Command-line client for the weather assistant MCP server."""

from __future__ import annotations

import argparse
import asyncio
import json
import logging
import sys
from typing import Any, Iterable

from dotenv import load_dotenv
from fastmcp import Client
from fastmcp.client import StreamableHttpTransport
from langfuse import observe
from opentelemetry import trace
from opentelemetry.propagate import inject

from weather_assistant.config.tracing import setup_tracing


logger = logging.getLogger(__name__)


def extract_payload(result: Any) -> Any:
    """Normalize CallToolResult-like responses into plain Python data."""
    if result is None:
        return None

    structured = getattr(result, "structured_content", None)
    if structured:
        return structured

    content = getattr(result, "content", None)
    if content:
        first = content[0] if len(content) > 0 else None
        if first is None:
            return []
        if hasattr(first, "text") and first.text is not None:
            try:
                return json.loads(first.text)
            except json.JSONDecodeError:
                return first.text
        return first

    return result


@observe
async def handle_weather_request(
    location: str,
    forecast_days: int = 3,
    backend_url: str = "http://localhost:8001/weather",
) -> tuple[Any, Any | None]:
    """Call MCP tools exposed by the Rust backend."""

    logger.debug(
        "Preparing MCP request",
        extra={"location": location, "forecast_days": forecast_days, "backend_url": backend_url},
    )

    # Prepare carrier for context propagation
    carrier: dict[str, str] = {}
    inject(carrier)

    # Create transport with trace context headers
    transport = StreamableHttpTransport(url=backend_url, headers=carrier)

    async with Client(transport) as client:
        # Get current weather
        logger.debug("Calling get_weather tool")
        weather_result = await client.call_tool("get_weather", {"location": location})

        # Get forecast if requested
        forecast_result: Any | None = None
        if forecast_days > 0:
            logger.debug("Calling get_forecast tool", extra={"forecast_days": forecast_days})
            forecast_result = await client.call_tool(
                "get_forecast", {"location": location, "days": forecast_days}
            )
        
        logger.debug("Received weather responses")
        return weather_result, forecast_result


def _ensure_iterable_forecast(data: Any) -> Iterable[Any]:
    """Return an iterable of forecast entries regardless of backend shape."""
    if data is None:
        return []
    if isinstance(data, dict) and "items" in data:
        return data["items"]
    if isinstance(data, list):
        return data
    return [data]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("location", help="City name to request weather information for")
    parser.add_argument(
        "--forecast-days",
        "-d",
        type=int,
        default=3,
        help="Number of forecast days to request (0 for none)",
    )
    parser.add_argument(
        "--backend-url",
        default="http://localhost:8001/weather",
        help="MCP HTTP endpoint exposed by the Rust server",
    )
    parser.add_argument(
        "--log-level",
        default="INFO",
        help="Logging level (DEBUG, INFO, WARNING, ERROR, CRITICAL)",
    )

    args = parser.parse_args(argv)

    logging.basicConfig(
        level=getattr(logging, args.log_level.upper(), logging.INFO),
        format="%(asctime)s %(name)s [%(levelname)s] %(message)s",
    )
    logger.debug("Initialized logging", extra={"log_level": args.log_level})

    # Load environment and initialize tracing once
    load_dotenv()
    _langfuse_client = setup_tracing()
    tracer = trace.get_tracer(__name__)

    logger.info(
        "Requesting weather",
        extra={
            "location": args.location,
            "forecast_days": args.forecast_days,
            "backend_url": args.backend_url,
        },
    )

    with tracer.start_as_current_span("weather_request") as span:
        span.set_attribute("location", args.location)
        span.set_attribute("forecast_days", args.forecast_days)
        span.set_attribute("backend_url", args.backend_url)

        try:
            weather_result, forecast_result = asyncio.run(
                handle_weather_request(args.location, args.forecast_days, backend_url=args.backend_url)
            )
        except Exception as exc:  # pragma: no cover - defensive guard for CLI errors
            logger.exception("Weather request failed")
            print(f"Error fetching weather: {exc}", file=sys.stderr)
            span.record_exception(exc)
            span.set_status(trace.StatusCode.ERROR)
            return 1

    weather_data = extract_payload(weather_result) or {}
    if isinstance(weather_data, str):
        try:
            weather_data = json.loads(weather_data)
        except json.JSONDecodeError:
            print(weather_data)
            return 0

    print(f"Current weather in {args.location}:")
    temperature = weather_data.get("temperature")
    if temperature is not None:
        print(f"  Temperature: {temperature}°C")
    condition = weather_data.get("condition")
    if condition is not None:
        print(f"  Condition: {condition}")
    humidity = weather_data.get("humidity")
    if humidity is not None:
        print(f"  Humidity: {humidity}%")

    if args.forecast_days > 0 and forecast_result is not None:
        raw_forecast = extract_payload(forecast_result) or []
        if isinstance(raw_forecast, str):
            try:
                raw_forecast = json.loads(raw_forecast)
            except json.JSONDecodeError:
                print("\nForecast:")
                print(raw_forecast)
                return 0

        print(f"\n{args.forecast_days}-day forecast:")
        for entry in _ensure_iterable_forecast(raw_forecast):
            day = entry.get("day", "?")
            high = entry.get("high")
            low = entry.get("low")
            condition = entry.get("condition")
            precip = entry.get("precipitation_chance")
            print(f"  Day {day}:")
            if high is not None and low is not None:
                print(f"    Temps: high {high}°C / low {low}°C")
            if condition is not None:
                print(f"    Condition: {condition}")
            if precip is not None:
                print(f"    Chance of rain: {precip}%")

    return 0


if __name__ == "__main__":
    sys.exit(main())
