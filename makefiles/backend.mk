BACKEND_RUN ?= $(COMPOSE) run --rm --no-TTY backend

.PHONY: back-ci
back-ci: back-format-check back-lint back-test back-audit

.PHONY: back-lint
back-lint:
	@echo "Linting backend code..."
	@$(BACKEND_RUN) cargo clippy --locked --all-targets --all-features -- -D warnings

.PHONY: back-check
back-check:
	@echo "Checking backend code..."
	@$(BACKEND_RUN) cargo check --locked --all-targets --all-features

.PHONY: back-format
back-format:
	@echo "Formatting backend code..."
	@$(BACKEND_RUN) cargo fmt

.PHONY: back-format-check
back-format-check:
	@echo "Checking backend code formatting..."
	@$(BACKEND_RUN) cargo fmt -- --check

.PHONY: back-test
back-test:
	@echo "Running backend tests..."
	@$(BACKEND_RUN) cargo test --locked --all-targets --all-features

.PHONY: back-audit
back-audit:
	@echo "Auditing backend dependencies..."
	@$(BACKEND_RUN) cargo audit --deny warnings

.PHONY: back-clean
back-clean:
	@echo "Removing backend build artifacts..."
	@$(BACKEND_RUN) cargo clean

.PHONY: back-lock
back-lock:
	@echo "Locking backend dependencies..."
	@$(BACKEND_RUN) cargo generate-lockfile

.PHONY: back-sync
back-sync:
	@echo "Fetching backend dependencies..."
	@$(BACKEND_RUN) cargo fetch --locked

.PHONY: back-update-deps
back-update-deps:
	@echo "Updating backend dependencies..."
	@$(BACKEND_RUN) cargo update $(foreach PACKAGE,$(ARGS),--package $(PACKAGE))

.PHONY: back-install
back-install: back-lock back-sync

.PHONY: back-sh
back-sh:
	@$(COMPOSE) run -it backend bash
