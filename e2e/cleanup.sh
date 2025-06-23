#!/bin/bash

set -e

E2E_IMAGE_NAME="mus:e2e-test"
E2E_CONTAINER_NAME="mus-e2e-test"

echo "Cleaning up E2E resources..."

if docker ps -q -f name="$E2E_CONTAINER_NAME" | grep -q .; then
    echo "Stopping container $E2E_CONTAINER_NAME..."
    docker stop "$E2E_CONTAINER_NAME" 2>/dev/null || true
fi

if docker ps -aq -f name="$E2E_CONTAINER_NAME" | grep -q .; then
    echo "Removing container $E2E_CONTAINER_NAME..."
    docker rm "$E2E_CONTAINER_NAME" 2>/dev/null || true
fi

if docker images -q "$E2E_IMAGE_NAME" | grep -q .; then
    echo "Removing image $E2E_IMAGE_NAME..."
    docker rmi "$E2E_IMAGE_NAME" 2>/dev/null || true
fi

echo "E2E cleanup complete"
