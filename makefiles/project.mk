.PHONY: ci
ci: markdown-lint back-lint front-lint e2e-lint front-svelte-check front-test back-test e2e-test-headless

.PHONY: markdown-lint
markdown-lint:
	markdownlint-cli2
