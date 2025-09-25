"""Streamlit client for weather assistant with distributed tracing."""

import streamlit as st
from dotenv import load_dotenv
from fastmcp import Client
from fastmcp.client import StreamableHttpTransport
from langfuse import observe
from opentelemetry import trace
from opentelemetry.propagate import inject

from weather_assistant.config.tracing import setup_tracing


def extract_payload(result):
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
            import json

            try:
                return json.loads(first.text)
            except json.JSONDecodeError:
                return first.text
        return first

    return result

# Load environment variables
load_dotenv()

# Initialize tracing
langfuse_client = setup_tracing()
tracer = trace.get_tracer(__name__)


@observe
async def handle_weather_request(location: str, forecast_days: int = 3):
    """Handle weather request with distributed tracing."""

    # Prepare carrier for context propagation
    carrier: dict[str, str] = {}
    inject(carrier)

    # Connect to Rust MCP server
    backend_url = "http://localhost:8001/weather"

    # Create transport with trace context headers
    transport = StreamableHttpTransport(url=backend_url, headers=carrier)

    async with Client(transport) as client:
        # Get current weather
        weather_result = await client.call_tool("get_weather", {"location": location})

        # Get forecast if requested
        forecast_result = None
        if forecast_days > 0:
            forecast_result = await client.call_tool("get_forecast", {"location": location, "days": forecast_days})

        return weather_result, forecast_result


# Streamlit UI
st.title("🌤️ Weather Assistant")
st.caption("Connected to Rust MCP Server on port 8001")

# User input
location = st.text_input("Enter city name:", "San Francisco")
forecast_days = st.slider("Forecast days:", 0, 7, 3)

if st.button("Get Weather"):
    with st.spinner("Fetching weather data..."):
        # Create a root span for the entire operation
        with tracer.start_as_current_span("weather_request") as span:
            span.set_attribute("location", location)
            span.set_attribute("forecast_days", forecast_days)

            try:
                import asyncio

                # Run the async function
                weather, forecast = asyncio.run(handle_weather_request(location, forecast_days))

                weather_data = extract_payload(weather) or {}
                if isinstance(weather_data, str):
                    import json

                    weather_data = json.loads(weather_data)

                # Display current weather
                st.subheader(f"Current Weather in {location}")
                col1, col2, col3 = st.columns(3)
                with col1:
                    st.metric("Temperature", f"{weather_data['temperature']}°C")
                with col2:
                    st.metric("Condition", weather_data["condition"])
                with col3:
                    st.metric("Humidity", f"{weather_data['humidity']}%")

                # Display forecast if requested
                if forecast and forecast_days > 0:
                    forecast_data = extract_payload(forecast) or []

                    # Accept either {'items': [...]} or raw list
                    if isinstance(forecast_data, dict) and "items" in forecast_data:
                        forecast_items = forecast_data["items"]
                    else:
                        forecast_items = forecast_data

                    st.subheader(f"{forecast_days}-Day Forecast")
                    for day_forecast in forecast_items:
                        with st.expander(f"Day {day_forecast['day']}"):
                            col1, col2, col3 = st.columns(3)
                            with col1:
                                st.write(f"High: {day_forecast['high']}°C")
                                st.write(f"Low: {day_forecast['low']}°C")
                            with col2:
                                st.write(f"Condition: {day_forecast['condition']}")
                            with col3:
                                st.write(f"Rain chance: {day_forecast['precipitation_chance']}%")

            except Exception as e:
                st.error(f"Error fetching weather: {str(e)}")
                span.record_exception(e)
                span.set_status(trace.StatusCode.ERROR)
