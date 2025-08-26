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
	@$(DOCKER_COMPOSE_CMD) down --remove-orphans --volumes

.PHONY: logs
logs:
	@$(DOCKER_COMPOSE_CMD) logs --tail=5000 $(ARGS)

.PHONY: build
build:
	@$(MAKE) down
	@echo "Removing app_data_database and app_data_covers volumes..."
	@docker volume rm $$(docker volume ls -q | grep -E "(app_data_database|app_data_covers)") 2>/dev/null || true
	@$(DOCKER_COMPOSE_CMD) build --pull
	@$(MAKE) up

.PHONY: ps
ps:
	@$(DOCKER_COMPOSE_CMD) ps
