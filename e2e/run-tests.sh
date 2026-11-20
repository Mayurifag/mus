#!/bin/bash

set -e

E2E_IMAGE_NAME="${E2E_IMAGE_NAME:-mus:e2e-test}"
export E2E_CONTAINER_NAME="${E2E_CONTAINER_NAME:-mus-e2e-test}"
E2E_HOST_PORT="4124"
E2E_TIMEOUT=12

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE="${1:-headless}"
E2E_MUSIC_DIR=""
container_created=false

case "$MODE" in
headless | headed | debug) ;;
*)
  echo "Usage: $0 [headless|headed|debug]"
  exit 1
  ;;
esac

cleanup() {
  status=$?
  if [ "$container_created" = true ]; then
    docker rm -f "$E2E_CONTAINER_NAME" >/dev/null || status=1
  fi
  if [ -n "$E2E_MUSIC_DIR" ]; then
    rm -rf -- "$E2E_MUSIC_DIR" || status=1
  fi
  exit "$status"
}

trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

if docker container inspect "$E2E_CONTAINER_NAME" >/dev/null 2>&1; then
  echo "Container $E2E_CONTAINER_NAME already exists; run make e2e-clean before retrying."
  exit 1
fi

E2E_MUSIC_DIR="$(mktemp -d)"
export E2E_MUSIC_DIR
cp -R "$SCRIPT_DIR/original/." "$E2E_MUSIC_DIR/"

echo "Installing Playwright dependencies..."
cd "$SCRIPT_DIR"
npm ci --no-audit --no-fund --prefer-offline
npx playwright install chromium

echo "Starting container..."
docker create --name "$E2E_CONTAINER_NAME" \
  -p "127.0.0.1:$E2E_HOST_PORT:8000" \
  -e WATCHFILES_FORCE_POLLING=true \
  -v "$E2E_MUSIC_DIR:/app_data/music:rw" \
  "$E2E_IMAGE_NAME"
container_created=true
docker start "$E2E_CONTAINER_NAME"

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

echo "Running E2E tests..."
cd "$SCRIPT_DIR"
export NODE_OPTIONS="${NODE_OPTIONS:+$NODE_OPTIONS }--disable-warning=DEP0205"
case "$MODE" in
headed) npx playwright test --headed ;;
debug) npx playwright test --debug ;;
*) npx playwright test ;;
esac
