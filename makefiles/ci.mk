.PHONY: ci
ci:
	@$(MAKE) markdown-lint && \
	$(MAKE) back-ci && \
	$(MAKE) front-lint && \
	$(MAKE) e2e-lint && \
	$(MAKE) front-svelte-check && \
	$(MAKE) front-test

.PHONY: full-ci
full-ci:
	$(MAKE) ci && \
	$(MAKE) e2e

.PHONY: markdown-lint
markdown-lint:
	@$(COMPOSE) run --rm -v "$(PWD):/workspace" -w /workspace frontend npx markdownlint-cli2
