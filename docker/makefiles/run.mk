DOCKER_COMPOSE_CMD := docker compose -f docker/docker-compose.yml

.PHONY: up stop ci

up:
	@$(DOCKER_COMPOSE_CMD) up --remove-orphans -d

stop:
	@$(DOCKER_COMPOSE_CMD) down

ci: format lint test

sh:
	@$(DOCKER_COMPOSE_CMD) run --rm --remove-orphans mus bash
