DOCKER_COMPOSE_CMD := docker compose -f docker/docker-compose.yml -f docker/docker-compose.override.yml
DOCKER_PROD_CMD := docker build -f docker/production/production.Dockerfile

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

.PHONY: docker-test-prod
docker-test-prod:
	@echo "Testing production Docker image..."
	@echo "Building production image..."
	@$(DOCKER_PROD_CMD) -t mus:test .
	@echo "Starting production container with music folder from override config..."
	@MUSIC_PATH=$$(grep -A 15 "backend:" docker/docker-compose.override.yml | grep "/app/music" | sed 's/.*- \([^:]*\):.*/\1/' | xargs) && \
	echo "Using music path: $$MUSIC_PATH" && \
	docker run -d --name mus-prod-test -p 4124:8000 \
		-v "$$MUSIC_PATH":/app/music \
		mus:test
	@echo "Waiting for container to start..."
	@sleep 10
	@echo "Production container started at http://localhost:4124"
	@echo "Use 'make docker-test-prod-stop' to stop and clean up"

.PHONY: docker-test-prod-stop
docker-test-prod-stop:
	@echo "Stopping and removing production test container..."
	@docker stop mus-prod-test 2>/dev/null || true
	@docker rm mus-prod-test 2>/dev/null || true
	@docker rmi mus:test 2>/dev/null || true
	@echo "Production test cleanup complete"

.PHONY: back-sh
back-sh:
	@$(DOCKER_COMPOSE_CMD) exec backend bash
