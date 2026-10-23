.PHONY: front-install
front-install:
	@$(COMPOSE) run --no-TTY --rm frontend npm install --no-fund

.PHONY: front-reinstall
front-reinstall:
	@$(COMPOSE) run --no-TTY --rm frontend sh -c 'find /app/node_modules -mindepth 1 -maxdepth 1 -exec rm -rf -- {} + && npm install --no-fund'

.PHONY: front-npm-install
front-npm-install:
	@$(COMPOSE) run --no-TTY --rm frontend npm install --no-fund $(ARGS)

.PHONY: front-npm-dev-install
front-npm-dev-install:
	@$(COMPOSE) run --no-TTY --rm frontend npm install --no-fund -D $(ARGS)

.PHONY: front-npm-uninstall
front-npm-uninstall:
	@$(COMPOSE) run --no-TTY --rm frontend npm uninstall $(ARGS)

.PHONY: front-npm-update
front-npm-update:
	@$(COMPOSE) run --no-TTY --rm frontend npm update

.PHONY: front-npm-outdated
front-npm-outdated:
	@$(COMPOSE) run --no-TTY --rm frontend npm outdated

.PHONY: front-audit
front-audit:
	@$(COMPOSE) run --no-TTY --rm frontend npm audit

.PHONY: front-build
front-build:
	@$(COMPOSE) run --no-TTY --rm frontend npm run build

.PHONY: front-lint
front-lint:
	@$(COMPOSE) run --no-TTY --rm frontend sh -c "rm -rf coverage/ && npm run lint"

.PHONY: front-lint-only
front-lint-only: front-lint

.PHONY: front-format
front-format:
	@$(COMPOSE) run --no-TTY --rm frontend npm run format

.PHONY: front-test
front-test:
	@$(COMPOSE) run --no-TTY --rm frontend npm run test

.PHONY: front-svelte-check
front-svelte-check:
	@$(COMPOSE) run --no-TTY --rm frontend npm run check

.PHONY: front-ci-install
front-ci-install:
	@$(COMPOSE) run --no-TTY --rm frontend npm ci --no-fund
