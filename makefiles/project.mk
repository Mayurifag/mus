.PHONY: up
up:
	@$(COMPOSE) up --remove-orphans -d

.PHONY: down
down:
	@$(COMPOSE) down --remove-orphans

.PHONY: down-volumes
down-volumes:
	@$(COMPOSE) down --volumes --remove-orphans

.PHONY: logs
logs:
	@$(COMPOSE) logs --tail=5000 $(ARGS)

.PHONY: build
build:
	@$(MAKE) down
	@$(COMPOSE) build --pull
	@$(MAKE) up

.PHONY: rebuild-backend-image
rebuild-backend-image:
	@$(COMPOSE) build --pull backend

.PHONY: rebuild-backend-image-fresh
rebuild-backend-image-fresh:
	@$(COMPOSE) build --pull --no-cache backend

.PHONY: rebuild-frontend-image
rebuild-frontend-image:
	@$(COMPOSE) build --pull frontend

.PHONY: rebuild-frontend-image-fresh
rebuild-frontend-image-fresh:
	@$(COMPOSE) build --pull --no-cache frontend

.PHONY: prod-image
prod-image:
	@docker build -f docker/production/production.Dockerfile --pull -t mus:latest .

.PHONY: prod-image-fresh
prod-image-fresh:
	@docker build -f docker/production/production.Dockerfile --pull --no-cache -t mus:latest .

.PHONY: ps
ps:
	@$(COMPOSE) ps
