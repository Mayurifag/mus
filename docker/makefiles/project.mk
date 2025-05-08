DOCKER_COMPOSE_CMD := docker compose -f docker/docker-compose.yml

.PHONY: ci
ci: back-format back-lint back-test

.PHONY: build
build:
	@$(DOCKER_COMPOSE_CMD) build

.PHONY: up
up:
	@$(DOCKER_COMPOSE_CMD) up --remove-orphans -d

.PHONY: stop
stop:
	@$(DOCKER_COMPOSE_CMD) down
