BACKEND_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/../backend)

.PHONY: back-lint
back-lint: back-format
	@echo "Linting backend code..."
	cd $(BACKEND_DIR) && uv run ruff check src tests

.PHONY: back-format
back-format:
	@echo "Formatting backend code..."
	cd $(BACKEND_DIR) && uv run ruff format src tests

.PHONY: back-test
back-test:
	@echo "Running backend tests..."
	cd $(BACKEND_DIR) && uv run pytest tests
	cd $(BACKEND_DIR) && rm -rf MagicMock/

.PHONY: back-dev
back-dev:
	@echo "Starting backend development server on http://0.0.0.0:8000 ..."
	cd $(BACKEND_DIR) && uv run uvicorn src.mus.main:app --host 0.0.0.0 --port 8000 --reload --reload-dir src

.PHONY: back-uv-init
back-uv-init:
	@echo "Initializing uv environment in backend..."
	cd $(BACKEND_DIR) && uv init

.PHONY: back-remove-venv
back-remove-venv:
	@echo "Removing backend virtual environment..."
	cd $(BACKEND_DIR) && rm -rf .venv

.PHONY: back-create-venv
back-create-venv:
	@echo "Creating backend virtual environment..."
	cd $(BACKEND_DIR) && uv venv

.PHONY: back-activate-venv
back-activate-venv:
	@echo "To activate backend virtual environment, run: cd $(BACKEND_DIR) && . .venv/bin/activate"
	@echo "This command itself does not activate it in the current shell."

.PHONY: back-lock
back-lock:
	@echo "Locking backend dependencies..."
	cd $(BACKEND_DIR) && uv pip compile pyproject.toml -o requirements.txt --all-extras

.PHONY: back-sync
back-sync:
	@echo "Syncing backend dependencies..."
	cd $(BACKEND_DIR) && uv pip sync requirements.txt

.PHONY: back-venv-reprovision
back-venv-reprovision: back-remove-venv back-create-venv back-lock back-sync
	@echo "Backend virtual environment reprovisioned."

.PHONY: back-update-deps
back-update-deps:
	@echo "Updating backend dependencies..."
	cd $(BACKEND_DIR) && uv pip compile pyproject.toml -o requirements.txt --all-extras --upgrade
	$(MAKE) back-sync
	@echo "Backend dependencies updated successfully."

.PHONY: back-uv-install
# Example: make back-uv-install ARGS="pytest pytest-asyncio"
back-uv-install:
	@echo "Installing backend uv package(s): $(ARGS)..."
	cd $(BACKEND_DIR) && uv add $(ARGS)
	$(MAKE) back-lock
	$(MAKE) back-sync
	@echo "Backend package(s) $(ARGS) installed and dependencies updated."

.PHONY: back-install
back-install: back-lock back-sync
	@echo "Backend dependencies installed/synced."
