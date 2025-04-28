.PHONY: run-dev stop

run-dev:
	@echo "Starting development server..."
	@docker compose -f docker/docker-compose.yml up --build

stop:
	@echo "Stopping containers..."
	@docker compose -f docker/docker-compose.yml down
