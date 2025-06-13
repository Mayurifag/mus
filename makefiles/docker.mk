DOCKER_COMPOSE_CMD := docker compose -f docker/docker-compose.yml -f docker/docker-compose.override.yml
DOCKER_PROD_CMD := docker build -f docker/production.Dockerfile

.PHONY: up
up:
	@$(DOCKER_COMPOSE_CMD) up --remove-orphans -d

.PHONY: build
build:
	@$(DOCKER_COMPOSE_CMD) build --pull

.PHONY: down
down:
	@$(DOCKER_COMPOSE_CMD) down

.PHONY: logs
logs:
	@$(DOCKER_COMPOSE_CMD) logs --tail=5000 $(ARGS)

.PHONY: rebuild
rebuild: down build up

.PHONY: ps
ps:
	@$(DOCKER_COMPOSE_CMD) ps

.PHONY: docker-build-prod
docker-build-prod:
	@$(DOCKER_PROD_CMD) -t mus:latest .

.PHONY: docker-run-prod
docker-run-prod:
	docker run -p 4123:8000 -v $(shell pwd)/data:/app/data -v $(shell pwd)/music:/app/music mus:latest

.PHONY: back-sh
back-sh:
	@$(DOCKER_COMPOSE_CMD) exec backend bash
