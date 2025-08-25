DOCKER_COMPOSE_BACKEND_CMD := $(DOCKER_COMPOSE_CMD) run --rm --no-TTY backend

.PHONY: back-lint
back-lint: back-format
	@echo "Linting backend code..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv run ruff check src tests --no-fix
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv run vulture src tests --min-confidence 90
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv run bandit -r src

.PHONY: back-ruff-fix
back-ruff-fix:
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv run ruff check src tests --fix

.PHONY: back-format
back-format:
	@echo "Formatting backend code..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv run ruff format src tests

.PHONY: back-format-check
back-format-check:
	@echo "Checking backend code formatting..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv run ruff format src tests --check

.PHONY: back-test
back-test:
	@echo "Running backend tests..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) sh -c "rm -f /tmp/test.db && rm -rf /tmp/test_data && mkdir -p /tmp/test_data/database /tmp/test_data/covers /tmp/test_data/music && uv run pytest tests --tb=short && rm -rf MagicMock/ /tmp/test.db /tmp/test_data"

.PHONY: back-uv-init
back-uv-init:
	@echo "Initializing uv environment in backend..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv init

.PHONY: back-remove-venv
back-remove-venv:
	@echo "Removing backend virtual environment..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) rm -rf .venv

.PHONY: back-create-venv
back-create-venv:
	@echo "Creating backend virtual environment..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv venv

.PHONY: back-lock
back-lock:
	@echo "Locking backend dependencies..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv lock

.PHONY: back-sync
back-sync:
	@echo "Syncing backend dependencies..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv sync --all-extras

.PHONY: back-venv-reprovision
back-venv-reprovision: back-remove-venv back-create-venv back-lock back-sync

.PHONY: back-update-deps
back-update-deps:
	@echo "Updating backend dependencies..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv lock --upgrade
	$(MAKE) back-sync

.PHONY: back-uv-install
back-uv-install:
	@echo "Installing backend uv package(s): $(ARGS)..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv add $(ARGS)
	$(MAKE) back-install

.PHONY: back-install
back-install: back-lock back-sync

.PHONY: back-sh
back-sh:
	@$(DOCKER_COMPOSE_CMD) run -it backend bash
