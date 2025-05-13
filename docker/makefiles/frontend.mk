FRONTEND_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/../../frontend)

.PHONY: front-install
front-install:
	cd $(FRONTEND_DIR) && npm install

.PHONY: front-npm-install
front-npm-install:
	cd $(FRONTEND_DIR) && npm install $(ARGS)

.PHONY: front-dev
front-dev:
	cd $(FRONTEND_DIR) && npm run dev

.PHONY: front-build
front-build:
	cd $(FRONTEND_DIR) && npm run build

.PHONY: front-lint
front-lint:
	cd $(FRONTEND_DIR) && npm run lint

.PHONY: front-format
front-format:
	cd $(FRONTEND_DIR) && npm run format

.PHONY: front-test
front-test:
	cd $(FRONTEND_DIR) && npm run test
