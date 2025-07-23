#!/bin/bash

set -e

E2E_IMAGE_NAME="mus:e2e-test"
E2E_CONTAINER_NAME="mus-e2e-test"
E2E_HOST_PORT="4124"
E2E_SECRET_KEY="e2e-secret-key"
E2E_TIMEOUT=12

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
MODE="${1:-headless}"

case "$MODE" in
    headless|headed|debug) ;;
    *) echo "Usage: $0 [headless|headed|debug]"; exit 1 ;;
esac

cleanup() {
    if docker ps -q -f name="$E2E_CONTAINER_NAME" | grep -q .; then
        docker stop "$E2E_CONTAINER_NAME" 2>/dev/null || true
    fi
}

trap cleanup EXIT INT TERM

if docker ps -aq -f name="$E2E_CONTAINER_NAME" | grep -q .; then
    echo "Removing existing container..."
    docker stop "$E2E_CONTAINER_NAME" 2>/dev/null || true
    docker rm "$E2E_CONTAINER_NAME" 2>/dev/null || true
fi

echo "Building production Docker image..."
cd "$PROJECT_ROOT"
docker build -f docker/production/production.Dockerfile -t "$E2E_IMAGE_NAME" .

echo "Starting container..."
docker run -d --name "$E2E_CONTAINER_NAME" \
    -p "$E2E_HOST_PORT:8000" \
    -e SECRET_KEY="$E2E_SECRET_KEY" \
    -e WATCHFILES_FORCE_POLLING=true \
    -v "$SCRIPT_DIR/music:/app_data/music:rw" \
    "$E2E_IMAGE_NAME"

echo "Waiting for application to be ready..."
timeout=$E2E_TIMEOUT

while [ $timeout -gt 0 ]; do
    if ! docker ps -q -f name="$E2E_CONTAINER_NAME" | grep -q .; then
        echo "Container stopped unexpectedly"
        exit 1
    fi

    status=$(docker inspect --format="{{.State.Health.Status}}" "$E2E_CONTAINER_NAME" 2>/dev/null || echo "unknown")

    if [ "$status" = "unhealthy" ]; then
        echo "Container is unhealthy"
        docker logs "$E2E_CONTAINER_NAME"
        exit 1
    fi

    if [ "$status" = "healthy" ]; then
        health_response=$(curl -s http://localhost:$E2E_HOST_PORT/api/healthcheck.json 2>/dev/null || echo "")
        if echo "$health_response" | grep -q '"status":"healthy"'; then
            echo "Application ready"
            break
        fi
    fi

    sleep 2
    timeout=$((timeout - 2))
done

if [ $timeout -le 0 ]; then
    echo "Application failed to become ready within $E2E_TIMEOUT seconds"
    docker logs "$E2E_CONTAINER_NAME"
    exit 1
fi

echo "Installing Playwright dependencies..."
cd "$SCRIPT_DIR"
npm ci --no-audit --no-fund --prefer-offline

echo "Running E2E tests..."
case "$MODE" in
    headed) npx playwright test --headed ;;
    debug) npx playwright test --debug ;;
    *) npx playwright test ;;
esac

echo "E2E tests completed successfully!"
