DOCKER_COMPOSE_BACKEND_CMD := $(DOCKER_COMPOSE_CMD) run --rm --no-TTY backend

.PHONY: back-lint
back-lint: back-format
	@echo "Linting backend code..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv run ruff check src tests --no-fix
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv run vulture src tests --min-confidence 90
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv run bandit -r src

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
	@$(DOCKER_COMPOSE_BACKEND_CMD) sh -c "rm -f test.db && rm -rf test_data/ && mkdir -p test_data/database test_data/covers test_data/music && uv run pytest tests --tb=short && rm -rf MagicMock/ test.db test_data/"

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
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv pip compile pyproject.toml -o requirements.txt --all-extras

.PHONY: back-sync
back-sync:
	@echo "Syncing backend dependencies..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv pip sync requirements.txt

.PHONY: back-venv-reprovision
back-venv-reprovision: back-remove-venv back-create-venv back-lock back-sync

.PHONY: back-update-deps
back-update-deps:
	@echo "Updating backend dependencies..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) uv pip compile pyproject.toml -o requirements.txt --all-extras --upgrade
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
