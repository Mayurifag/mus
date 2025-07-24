.PHONY: ci
ci: back-lint front-lint e2e-lint front-svelte-check front-test back-test e2e-test-headless
