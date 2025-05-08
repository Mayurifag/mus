BACKEND_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/../../backend)

.PHONY: back-lint
back-lint:
	cd $(BACKEND_DIR) && uv run ruff check src tests


.PHONY: back-format
back-format:
	cd $(BACKEND_DIR) && uv run ruff format src tests

.PHONY: back-test
back-test:
	cd $(BACKEND_DIR) && uv run pytest tests

.PHONY: back-uv-init
back-uv-init:
	cd $(BACKEND_DIR) && uv init

.PHONY: back-remove-venv
back-remove-venv:
	cd $(BACKEND_DIR) && rm -rf .venv

.PHONY: back-create-venv
back-create-venv:
	cd $(BACKEND_DIR) && uv venv

.PHONY: back-activate-venv
back-activate-venv:
	cd $(BACKEND_DIR) && . .venv/bin/activate

.PHONY: back-lock
back-lock:
	cd $(BACKEND_DIR) && uv pip compile pyproject.toml -o requirements.txt --all-extras

.PHONY: back-sync
back-sync:
	cd $(BACKEND_DIR) && uv pip sync requirements.txt

.PHONY: back-venv-reprovision
back-venv-reprovision: back-remove-venv back-create-venv back-activate-venv back-lock back-sync

.PHONY: back-update-deps
back-update-deps:
	cd $(BACKEND_DIR) && uv pip compile pyproject.toml -o requirements.txt --all-extras --upgrade && make back-sync && echo "Dependencies updated successfully"

.PHONY: back-uv-install
back-uv-install:
	cd $(BACKEND_DIR) && uv add $(ARGS)
	$(MAKE) back-lock
	$(MAKE) back-sync
