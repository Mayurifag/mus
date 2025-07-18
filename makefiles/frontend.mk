FRONTEND_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/../frontend)

.PHONY: front-install
front-install:
	cd $(FRONTEND_DIR) && npm install --no-fund

.PHONY: front-npm-install
front-npm-install:
	cd $(FRONTEND_DIR) && npm install --no-fund $(ARGS)

.PHONY: front-npm-dev-install
front-npm-dev-install:
	cd $(FRONTEND_DIR) && npm install --no-fund -D $(ARGS)

.PHONY: front-npm-uninstall
front-npm-uninstall:
	cd $(FRONTEND_DIR) && npm uninstall $(ARGS)

.PHONY: front-dev
front-dev:
	cd $(FRONTEND_DIR) && npm run dev

.PHONY: front-build
front-build:
	cd $(FRONTEND_DIR) && npm run build

.PHONY: front-lint
front-lint: front-format
	cd $(FRONTEND_DIR) && rm -rf coverage/ && npm run lint

.PHONY: front-lint-only
front-lint-only:
	cd $(FRONTEND_DIR) && npm run lint

.PHONY: front-format
front-format:
	cd $(FRONTEND_DIR) && npm run format

.PHONY: front-test
front-test:
	cd $(FRONTEND_DIR) && npm run test

.PHONY: front-svelte-check
front-svelte-check:
	cd $(FRONTEND_DIR) && npm run check

.PHONY: front-ci-install
front-ci-install:
	cd $(FRONTEND_DIR) && npm ci
