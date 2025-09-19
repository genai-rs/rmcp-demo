# Rust MCP HTTP Server with OpenTelemetry and Langfuse

This repository demonstrates a Rust implementation of an MCP (Model Context Protocol) HTTP server with distributed tracing using OpenTelemetry and Langfuse, designed to work with FastMCP Python clients.

## Features

- 🦀 **Rust MCP HTTP Server**: JSON-RPC implementation compatible with FastMCP clients
- 🔍 **Distributed Tracing**: OpenTelemetry integration for trace propagation
- 📊 **Langfuse Integration**: LLM observability with token usage and latency tracking
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
- Langfuse instance (local or cloud)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/genai-rs/rmcp-demo
cd rmcp-demo
```

2. Copy the environment file and configure:
```bash
cp .env.example .env
# Edit .env with your Langfuse credentials and server URL
# For Langfuse Cloud, use: LANGFUSE_HOST=https://cloud.langfuse.com
# For external server, use your server's URL
```

3. Build the Rust backend:
```bash
cargo build --release
```

4. Install Python dependencies with uv:
```bash
# Install uv if you haven't already
curl -LsSf https://astral.sh/uv/install.sh | sh

# Create virtual environment and install dependencies
uv venv
uv pip install -e .

# Or simply use uv sync if you want to sync with lock file
uv sync
```

## Running

### Start the Rust MCP Server

```bash
cargo run
```

The server will start on `http://localhost:8001/weather`

### Start the Python Client

```bash
# Using uv run
uv run streamlit run weather_assistant/client.py

# Or activate the virtual environment first
source .venv/bin/activate  # On Unix/macOS
# or
.venv\Scripts\activate     # On Windows
streamlit run weather_assistant/client.py
```

### (Optional) Start the Python MCP Server

```bash
# Using uv run
uv run weather-server

# Or using module directly
uv run python -m weather_assistant.server
```

The Python server will start on `http://localhost:8000/weather`

## Configuration

### Environment Variables

- `LANGFUSE_PUBLIC_KEY`: Your Langfuse public key
- `LANGFUSE_SECRET_KEY`: Your Langfuse secret key
- `LANGFUSE_HOST`: Langfuse host URL (default: `http://localhost:3000`)
- `OTEL_SERVICE_NAME`: Service name for traces (default: `weather-assistant-rust`)
- `OPENAI_API_KEY`: OpenAI API key for the client

## How It Works

### Trace Propagation

1. The Streamlit client creates a trace context and injects it into HTTP headers
2. The Rust server extracts the trace context from headers
3. All operations are tracked as spans under the parent trace
4. Traces are exported to Langfuse for visualization

### MCP Protocol

The server implements the MCP protocol with:
- `tools/list`: Returns available tools
- `tools/call`: Executes tool functions
  - `get_weather`: Get current weather for a location
  - `get_forecast`: Get weather forecast for multiple days

## Development

### Python Development with uv

```bash
# Install development dependencies
uv pip install -e ".[dev]"

# Run linting
uv run ruff check .

# Run type checking
uv run mypy weather_assistant

# Run Python tests
uv run pytest
```

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
- **OpenTelemetry SDK**: Direct integration with SimpleSpanProcessor for synchronous export
- **Langfuse Exporter**: Native Rust implementation for LLM observability

## Comparison with Python Implementation

The Rust implementation provides:
- ⚡ Better performance and lower resource usage
- 🔒 Memory safety guarantees
- 🚀 Native async/await with Tokio
- 📦 Single binary deployment

## License

MIT