DOCKER_COMPOSE_CMD := docker compose -f docker/docker-compose.yml

.PHONY: run-dev stop ci

run-dev:
	@echo "Starting development server..."
	@$(DOCKER_COMPOSE_CMD) up --build

stop:
	@echo "Stopping containers..."
	@$(DOCKER_COMPOSE_CMD) down

ci: format lint test

sh:
	@$(DOCKER_COMPOSE_CMD) run --rm --remove-orphans mus bash
