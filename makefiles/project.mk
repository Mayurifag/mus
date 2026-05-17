DOCKER_COMPOSE_CMD := USER_ID=$$(id -u) GROUP_ID=$$(id -g) docker compose -f docker/docker-compose.yml -f docker/docker-compose.override.yml
DOCKER_PROD_CMD := docker build -f docker/production/production.Dockerfile

.PHONY: up
up:
	@$(DOCKER_COMPOSE_CMD) up --remove-orphans -d

.PHONY: down
down:
	@$(DOCKER_COMPOSE_CMD) down --remove-orphans

.PHONY: down-volumes
down-volumes:
	@$(DOCKER_COMPOSE_CMD) down --volumes --remove-orphans

.PHONY: logs
logs:
	@$(DOCKER_COMPOSE_CMD) logs --tail=5000 $(ARGS)

.PHONY: build
build:
	@$(MAKE) down
	@$(DOCKER_COMPOSE_CMD) build --pull
	@$(MAKE) up

.PHONY: rebuild-backend-image
rebuild-backend-image:
	@$(DOCKER_COMPOSE_CMD) build --pull backend

.PHONY: rebuild-backend-image-fresh
rebuild-backend-image-fresh:
	@$(DOCKER_COMPOSE_CMD) build --pull --no-cache backend

.PHONY: rebuild-frontend-image
rebuild-frontend-image:
	@$(DOCKER_COMPOSE_CMD) build --pull --no-cache frontend

.PHONY: prod-image
prod-image:
	@$(DOCKER_PROD_CMD) --pull -t mus:latest .

.PHONY: prod-image-fresh
prod-image-fresh:
	@$(DOCKER_PROD_CMD) --pull --no-cache -t mus:latest .

.PHONY: ps
ps:
	@$(DOCKER_COMPOSE_CMD) ps
