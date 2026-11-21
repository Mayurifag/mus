FRONTEND_RUN ?= $(COMPOSE) run --no-TTY --rm frontend

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
	@$(FRONTEND_RUN) npm audit --omit=dev
	@$(FRONTEND_RUN) npm audit --audit-level=moderate

.PHONY: front-build
front-build:
	@$(FRONTEND_RUN) npm run build

.PHONY: front-lint
front-lint:
	@$(FRONTEND_RUN) npm run lint

.PHONY: front-lint-only
front-lint-only: front-lint

.PHONY: front-format
front-format:
	@$(COMPOSE) run --no-TTY --rm frontend npm run format

.PHONY: front-test
front-test:
	@$(FRONTEND_RUN) npm run test

.PHONY: front-svelte-check
front-svelte-check:
	@$(FRONTEND_RUN) npm run check

.PHONY: front-ci-install
front-ci-install:
	@$(FRONTEND_RUN) npm ci --no-fund
