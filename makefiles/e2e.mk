.PHONY: e2e
e2e:
	@docker build -f docker/production/production.Dockerfile -t mus:e2e-test .
	@./e2e/run-tests.sh

.PHONY: e2e-current-image
e2e-current-image:
	@docker image inspect mus:latest >/dev/null
	@docker tag mus:latest mus:e2e-test
	@./e2e/run-tests.sh

.PHONY: e2e-clean
e2e-clean:
	@./e2e/cleanup.sh

.PHONY: e2e-lint
e2e-lint:
	@echo "Running E2E TypeScript linting..."
	@cd e2e && npm ci --no-audit --no-fund --prefer-offline && npm run lint

.PHONY: e2e-dev-smoke
e2e-dev-smoke:
	@cd e2e && npm ci --no-audit --no-fund --prefer-offline && NODE_OPTIONS="--disable-warning=DEP0205" BASE_URL=http://localhost:5173 npx playwright test tests/dev-smoke.spec.ts

.PHONY: e2e-update-deps
e2e-update-deps:
	@cd e2e && npm update

.PHONY: e2e-report
e2e-report:
	if command -v xdg-open >/dev/null 2>&1; then \
	    xdg-open ./e2e/playwright-report/index.html; \
	else \
	    open ./e2e/playwright-report/index.html; \
	fi
