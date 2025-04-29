.PHONY: remove-venv create-venv activate-venv lock sync update-deps venv-reprovision

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
	@uv pip install --upgrade pip
	@uv pip install --upgrade -e ".[dev]"
	@uv pip compile pyproject.toml -o requirements.txt --all-extras --upgrade
	@uv pip sync requirements.txt
	@echo "Dependencies updated successfully"
