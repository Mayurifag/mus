.PHONY: test

test:
	@echo "Running tests..."
	@uv run pytest tests
