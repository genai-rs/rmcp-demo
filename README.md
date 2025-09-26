# Rust MCP HTTP Server with OpenTelemetry and Jaeger

This repository demonstrates a Rust implementation of an MCP (Model Context Protocol) HTTP server with distributed tracing using OpenTelemetry and Jaeger (via OTLP), designed to work with FastMCP Python clients. The Python client can optionally forward traces to Langfuse for LLM observability.

## Features

- 🦀 **Rust MCP HTTP Server**: JSON-RPC implementation compatible with FastMCP clients
- 🔍 **Distributed Tracing**: OpenTelemetry integration for trace propagation
- 📊 **Jaeger Integration**: OTLP exporter targets Jaeger by default (with optional Langfuse support on the Python side)
- 🎨 **Streamlit Frontend**: Python client application (from the original implementation)
- 🌤️ **Weather Tools**: Example MCP tools for weather data

## Architecture

The project contains:
- **Rust Backend** (`src/`): HTTP-based MCP server with JSON-RPC protocol on port 8001
- **Python Frontend** (`weather_assistant/`): Streamlit client application using FastMCP
- **Python Backend** (`weather_assistant/server.py`): Reference Python MCP server on port 8000 (optional)

### Why HTTP instead of stdio?

The official `rmcp` crate is designed for stdio-based communication, which works well for local tool integration. However, this project uses HTTP transport to:
- Work seamlessly with FastMCP Python clients
- Enable distributed tracing across network boundaries
- Support web-based clients like Streamlit
- Allow proper trace context propagation via HTTP headers

## Setup

### Prerequisites

- Rust (latest stable)
- Python 3.10+ (managed via `uv`)
- [uv](https://github.com/astral-sh/uv) - Fast Python package installer and resolver
- Jaeger all-in-one instance (local via Docker or remote)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/genai-rs/rmcp-demo
cd rmcp-demo
```

2. Copy the environment file and configure:
```bash
cp .env.example .env
```

3. Build the Rust backend:
```bash
cargo build --release
```

4. Install Python dependencies with uv:
```bash
uv sync
```

## Running

### Start the Rust MCP Server

```bash
cargo run
```

The server will start on `http://localhost:8001/weather`

### Run Jaeger Locally (optional)

```bash
docker run -d --name jaeger --restart unless-stopped \
  -e COLLECTOR_OTLP_ENABLED=true \
  -p 16686:16686 \
  -p 14268:14268 \
  -p 4318:4318 \
  jaegertracing/all-in-one:1.56
```

Open the Jaeger UI at <http://localhost:16686>. Configure `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318` (or the traces endpoint) before starting the Rust server or Python CLI to push spans into Jaeger.

### Start the Python Client

```bash
uv run streamlit run weather_assistant/client.py
```

## Configuration

### Environment Variables

- `OTEL_EXPORTER_OTLP_ENDPOINT`: Base OTLP endpoint (default: `http://localhost:4318`). Set to your Jaeger collector.
- `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`: Full traces endpoint (overrides the base endpoint).
- `OTEL_SERVICE_NAME`: Service name for traces (default: `weather-assistant-rust`).
- `OPENAI_API_KEY`: OpenAI API key for the client (optional).
- `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY`, `LANGFUSE_HOST`: Optional keys for the Python client if you still want to mirror traces into Langfuse.

## How It Works

### Trace Propagation

1. The Streamlit client creates a trace context and injects it into HTTP headers
2. The Rust server extracts the trace context from headers
3. All operations are tracked as spans under the parent trace
4. Traces are exported to Jaeger (and optionally Langfuse on the Python side) for visualization

### MCP Protocol

The server implements the MCP protocol with:
- `tools/list`: Returns available tools
- `tools/call`: Executes tool functions
  - `get_weather`: Get current weather for a location
  - `get_forecast`: Get weather forecast for multiple days

## Development

### Rust Development

```bash
# Run tests
cargo test

# Run with verbose output
cargo run --verbose

# Build for production
cargo build --release
```

## Architecture Decisions

- **rmcp**: Official Rust MCP SDK with HTTP transport support via StreamableHttpService
- **Axum**: High-performance async web framework for the HTTP server
- **Tower**: Middleware for CORS and other HTTP concerns
- **OpenTelemetry SDK**: Batched OTLP exporter for asynchronous export to Jaeger
- **Jaeger**: Default backend for trace visualization (via OTLP HTTP)

## License

MIT
