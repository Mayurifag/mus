.PHONY: run-dev stop ci

run-dev:
	@echo "Starting development server..."
	@docker compose -f docker/docker-compose.yml up --build

stop:
	@echo "Stopping containers..."
	@docker compose -f docker/docker-compose.yml down

ci: format lint test
