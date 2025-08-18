DOCKER_COMPOSE_FRONTEND_CMD := $(DOCKER_COMPOSE_CMD) run --no-TTY --rm frontend

.PHONY: front-install
front-install:
	@$(DOCKER_COMPOSE_FRONTEND_CMD) npm install --no-fund

.PHONY: front-npm-install
front-npm-install:
	@$(DOCKER_COMPOSE_FRONTEND_CMD) npm install --no-fund $(ARGS)

.PHONY: front-npm-dev-install
front-npm-dev-install:
	@$(DOCKER_COMPOSE_FRONTEND_CMD) npm install --no-fund -D $(ARGS)

.PHONY: front-npm-uninstall
front-npm-uninstall:
	@$(DOCKER_COMPOSE_FRONTEND_CMD) npm uninstall $(ARGS)

.PHONY: front-build
front-build:
	@$(DOCKER_COMPOSE_FRONTEND_CMD) npm run build

.PHONY: front-lint
front-lint: front-format
	@$(DOCKER_COMPOSE_FRONTEND_CMD) sh -c "rm -rf coverage/ && npm run lint"

.PHONY: front-lint-only
front-lint-only:
	@$(DOCKER_COMPOSE_FRONTEND_CMD) npm run lint

.PHONY: front-format
front-format:
	@$(DOCKER_COMPOSE_FRONTEND_CMD) npm run format

.PHONY: front-test
front-test:
	@$(DOCKER_COMPOSE_FRONTEND_CMD) npm run test

.PHONY: front-svelte-check
front-svelte-check:
	@$(DOCKER_COMPOSE_FRONTEND_CMD) npm run check

.PHONY: front-ci-install
front-ci-install:
	@$(DOCKER_COMPOSE_FRONTEND_CMD) npm ci
