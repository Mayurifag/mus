.PHONY: e2e-test-headless
e2e-test-headless:
	@./e2e/run-tests.sh

.PHONY: e2e-clean
e2e-clean:
	@./e2e/cleanup.sh

.PHONY: e2e-lint
e2e-lint:
	@echo "Building E2E Docker image..."
	@cd e2e && docker build -t mus:e2e-lint .
	@echo "Running E2E TypeScript linting..."
	@docker run --rm mus:e2e-lint npm run lint
