.PHONY: remove-venv create-venv activate-venv lock sync update-deps venv-reprovision uv-install

remove-venv:
	@rm -rf .venv

create-venv:
	@uv venv

activate-venv:
	@. .venv/bin/activate

lock:
	@echo "Locking dependencies..."
	@uv pip compile pyproject.toml -o requirements.txt --all-extras

sync:
	@echo "Syncing dependencies..."
	@uv pip sync requirements.txt

venv-reprovision: remove-venv create-venv activate-venv lock sync

update-deps:
	@echo "Updating all dependencies..."
	@uv pip compile pyproject.toml -o requirements.txt --all-extras --upgrade
	@make sync
	@echo "Dependencies updated successfully"

uv-install:
	@uv add $(ARGS)
	@make lock
	@make sync
