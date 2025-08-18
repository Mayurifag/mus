.PHONY: ci
ci: markdown-lint back-lint front-lint e2e-lint front-svelte-check front-test back-test e2e-test-headless

.PHONY: markdown-lint
markdown-lint:
	@$(DOCKER_COMPOSE_CMD) run --rm -v "$(PWD):/workspace" -w /workspace frontend npx markdownlint-cli2
