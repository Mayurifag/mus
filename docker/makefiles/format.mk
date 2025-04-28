.PHONY: format

format:
	@echo "Formatting..."
	@uv run ruff format src tests
