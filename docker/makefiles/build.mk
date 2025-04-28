.PHONY: build

build:
	@echo "Building Docker image..."
	@docker compose -f docker/docker-compose.yml build
