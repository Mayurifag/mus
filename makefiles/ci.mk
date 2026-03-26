.PHONY: ci
ci:
	@$(MAKE) markdown-lint && \
	$(MAKE) back-ruff-fix && \
	$(MAKE) back-lint && \
	$(MAKE) front-lint && \
	$(MAKE) e2e-lint && \
	$(MAKE) front-svelte-check && \
	$(MAKE) front-test && \
	$(MAKE) back-test

.PHONY: full-ci
full-ci:
	$(MAKE) ci && \
	$(MAKE) e2e

.PHONY: markdown-lint
markdown-lint:
	@$(DOCKER_COMPOSE_CMD) run --rm -v "$(PWD):/workspace" -w /workspace frontend npx markdownlint-cli2
