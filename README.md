# Mus - Personal Music Server

A modern personal music server with a web interface that allows you to stream your music collection from anywhere.

## Tech Stack

- **Backend**: FastAPI, SQLModel, Python 3.12+, async operations
- **Frontend**: SvelteKit, TypeScript, Tailwind CSS, shadcn-svelte
- **Storage**: SQLite database
- **Containerization**: Docker, Docker Compose

> **Note**: Tauri desktop app integration is planned for a future phase of development.

## Project Structure

```
mus/
├── backend/               # FastAPI backend service
│   ├── src/mus/           # Main application code
│   │   ├── domain/        # Domain entities
│   │   ├── application/   # Use cases and DTOs
│   │   └── infrastructure/# API, persistence, scanner implementations
│   └── tests/             # Backend tests
├── frontend/              # SvelteKit frontend application
│   ├── src/               # Frontend source code
│   │   ├── lib/           # Components, stores, services, types
│   │   └── routes/        # Application pages
│   └── static/            # Static assets
├── docker/                # Docker configuration
│   ├── makefiles/         # Makefile modules for each service
│   └── *.Dockerfile       # Service-specific Dockerfiles
└── data/                  # Application data (covers, database)
```

## Getting Started

### Prerequisites

- Python 3.12+
- Node.js 18+ and npm
- Docker and Docker Compose (optional, for containerized usage)
- Make
- uv (Python package manager)

### Setup

1. **Clone the repository**

```bash
git clone https://github.com/yourusername/mus.git
cd mus
```

2. **Backend Setup**

```bash
# Install Python dependencies
make back-uv-install

# Run the backend development server
make back-dev
```

3. **Frontend Setup**

```bash
# Install npm dependencies
make front-install

# Run the frontend development server
make front-dev
```

4. **Docker Setup (Alternative)**

```bash
# Build and start all services
make up

# View logs
make logs

# Stop services
make down
```

## Development Workflow

### Running Tests

```bash
# Run all tests
make test

# Run backend-specific tests
make back-test

# Run frontend-specific tests
make front-test
```

### Code Quality

```bash
# Run linters
make lint

# Format code
make format

# Run CI checks (lint, format, test)
make ci
```

### Adding Dependencies

```bash
# Add backend dependencies
make back-uv-install <package-name>

# Add frontend dependencies
make front-install <package-name>
```

## Usage

1. Start the backend and frontend servers
2. Access the web interface at http://localhost:5173
3. For first-time login via web browser, use: http://localhost:8000/api/v1/auth/login-via-secret/<SECRET_KEY>
4. Scan your music collection by using the scan button in the UI
5. Enjoy your music library with a responsive, modern interface!

## Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details.
