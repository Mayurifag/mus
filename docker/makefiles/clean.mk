.PHONY: clean

clean:
	@echo "Cleaning..."
	@find . -type f -name '*.pyc' -delete
	@find . -type d -name '__pycache__' -exec rm -rf {} +
	@find . -type d -name '.pytest_cache' -exec rm -rf {} +
	@find . -type d -name '.ruff_cache' -exec rm -rf {} +
	@find . -type d -name '.mypy_cache' -exec rm -rf {} +
	@find . -type d -name '*.egg-info' -exec rm -rf {} +
	@find . -type d -name 'dist' -exec rm -rf {} +
	@find . -type d -name 'build' -exec rm -rf {} +
