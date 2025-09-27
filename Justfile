rmcp:
    cargo run

watch:
     cargo watch -x run -w src

cli location="Brussels":
    uv run python weather_assistant/cli.py "{{location}}"