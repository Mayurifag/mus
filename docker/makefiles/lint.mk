.PHONY: lint lint-py lint-css lint-js

lint-py:
	@echo "Linting Python..."
	@uv run ruff check src tests

.PHONY: lint-css
lint-css:
	@echo "Linting CSS..."
	@bunx stylelint "src/mus/infrastructure/web/static/css/**/*.css" --fix

.PHONY: lint-js
lint-js:
	@echo "Linting JavaScript..."
	@bunx eslint "src/mus/infrastructure/web/static/js/**/*.js" --fix

lint: lint-py lint-css lint-js
