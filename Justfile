jaeger:
    docker run -it --rm --name jaeger \
      -e COLLECTOR_OTLP_ENABLED=true \
      -p 16686:16686 \
      -p 14268:14268 \
      -p 4318:4318 \
      jaegertracing/all-in-one:1.56

rmcp:
    cargo run

watch:
     cargo watch -x run -w src

cli:
    uv run python weather_assistant/cli.py ny