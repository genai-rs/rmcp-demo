# Rust MCP Demo with OpenTelemetry and Langfuse

This repository demonstrates a Rust implementation of an MCP (Model Context Protocol) server with distributed tracing using OpenTelemetry and Langfuse.

## Features

- 🦀 **Rust MCP Server**: Built with the official MCP Rust SDK
- 🔍 **Distributed Tracing**: OpenTelemetry integration for trace propagation
- 📊 **Langfuse Integration**: LLM observability with token usage and latency tracking
- 🎨 **Streamlit Frontend**: Python client application (from the original implementation)
- 🌤️ **Weather Tools**: Example MCP tools for weather data

## Architecture

The project contains:
- **Rust Backend** (`src/`): MCP server implementation on port 8001
- **Python Frontend** (`weather_assistant/`): Streamlit client application
- **Python Backend** (`weather_assistant/server.py`): Original Python MCP server on port 8000

## Setup

### Prerequisites

- Rust (latest stable)
- Python 3.10+
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
# Edit .env with your Langfuse credentials
```

3. Build the Rust backend:
```bash
cargo build --release
```

4. Install Python dependencies:
```bash
pip install -e .
```

## Running

### Start the Rust MCP Server

```bash
cargo run
```

The server will start on `http://localhost:8001/weather`

### Start the Python Client

```bash
streamlit run weather_assistant/client.py
```

### (Optional) Start the Python MCP Server

```bash
python -m weather_assistant.server
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

### Running Tests

```bash
cargo test
```

### Building for Production

```bash
cargo build --release
```

## Architecture Decisions

- **Axum**: High-performance async web framework for the HTTP server
- **Tower**: Middleware for CORS and other HTTP concerns
- **OpenTelemetry SDK**: Direct integration for maximum control over tracing
- **Langfuse Exporter**: Native Rust implementation for LLM observability

## Comparison with Python Implementation

The Rust implementation provides:
- ⚡ Better performance and lower resource usage
- 🔒 Memory safety guarantees
- 🚀 Native async/await with Tokio
- 📦 Single binary deployment

## License

MIT