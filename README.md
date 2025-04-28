# mus - Personal Music Server

A personal music server built with Python, FastAPI, and HTMX.

## Features

- Async Python backend with FastAPI
- HTMX for dynamic frontend interactions
- Structured logging with structlog
- Docker support for easy deployment
- Hexagonal architecture for clean code organization

## Setup

1. Create a virtual environment:
   ```bash
   uv venv
   ```

2. Install dependencies:
   ```bash
   make lock
   make sync
   ```

3. Build and run:
   ```bash
   make build
   make run-dev
   ```

## Development

- Run tests: `make test`
- Lint code: `make lint`
- Format code: `make format`
- Clean build artifacts: `make clean`

## Architecture

The project follows a hexagonal architecture pattern:

- `domain/`: Core business logic and entities
- `application/`: Use cases and application services
- `infrastructure/`: Adapters for external systems (web, persistence, etc.)

## Logging

The application uses structlog for structured JSON logging. Logs are written to stdout in JSON format.

## License

MIT
