.PHONY: lint

lint:
	@echo "Linting..."
	@uv run ruff check src tests
