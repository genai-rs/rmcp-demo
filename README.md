# Rust MCP HTTP Server with OpenTelemetry and Langfuse

This repository demonstrates a Rust implementation of an MCP (Model Context Protocol) HTTP server with distributed tracing using OpenTelemetry and Langfuse, designed to work with FastMCP Python clients.

## Features

- **Rust MCP HTTP server**: JSON-RPC implementation compatible with FastMCP clients
- **Distributed tracing**: OpenTelemetry integration for trace propagation
- **Langfuse integration**: exported traces for LLM observability
- **Streamlit frontend**: Python client application (from the original implementation)
- **Weather tools**: example MCP tools for weather data

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
- Langfuse account (cloud or self-hosted)

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

### Configure Langfuse

Set up your Langfuse credentials in the `.env` file:
- `LANGFUSE_PUBLIC_KEY`: Your Langfuse public key
- `LANGFUSE_SECRET_KEY`: Your Langfuse secret key
- `LANGFUSE_BASE_URL`: Langfuse endpoint (default: `https://cloud.langfuse.com`)

### Start the Python Client

```bash
uv run streamlit run weather_assistant/client.py
```

## Configuration

### Environment Variables

- `OTEL_SERVICE_NAME`: Service name for traces (default: `weather-assistant`).
- `LANGFUSE_PUBLIC_KEY`: Your Langfuse public key (required for tracing).
- `LANGFUSE_SECRET_KEY`: Your Langfuse secret key (required for tracing).
- `LANGFUSE_BASE_URL` or `LANGFUSE_HOST`: Langfuse endpoint (default: `https://cloud.langfuse.com`).
- `OPENAI_API_KEY`: OpenAI API key for the client (optional).

## How It Works

### Trace Propagation

1. The Streamlit client creates a trace context and injects it into HTTP headers
2. The Rust server extracts the trace context from headers
3. All operations are tracked as spans under the parent trace
4. Traces are exported to Langfuse for visualization and analysis

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
- **OpenTelemetry SDK**: Integration with Langfuse for trace export
- **Langfuse**: Backend for trace visualization and LLM observability

## License

MIT
