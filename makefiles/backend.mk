.PHONY: back-ci
back-ci: back-format-check back-lint back-test back-audit

.PHONY: back-lint
back-lint:
	@echo "Linting backend code..."
	@$(COMPOSE) run --rm --no-TTY backend cargo clippy --locked --all-targets --all-features -- -D warnings

.PHONY: back-check
back-check:
	@echo "Checking backend code..."
	@$(COMPOSE) run --rm --no-TTY backend cargo check --locked --all-targets --all-features

.PHONY: back-format
back-format:
	@echo "Formatting backend code..."
	@$(COMPOSE) run --rm --no-TTY backend cargo fmt

.PHONY: back-format-check
back-format-check:
	@echo "Checking backend code formatting..."
	@$(COMPOSE) run --rm --no-TTY backend cargo fmt -- --check

.PHONY: back-test
back-test:
	@echo "Running backend tests..."
	@$(COMPOSE) run --rm --no-TTY backend cargo test --locked --all-targets --all-features

.PHONY: back-audit
back-audit:
	@echo "Auditing backend dependencies..."
	@$(COMPOSE) run --rm --no-TTY backend cargo audit --deny warnings

.PHONY: back-clean
back-clean:
	@echo "Removing backend build artifacts..."
	@$(COMPOSE) run --rm --no-TTY backend cargo clean

.PHONY: back-lock
back-lock:
	@echo "Locking backend dependencies..."
	@$(COMPOSE) run --rm --no-TTY backend cargo generate-lockfile

.PHONY: back-sync
back-sync:
	@echo "Fetching backend dependencies..."
	@$(COMPOSE) run --rm --no-TTY backend cargo fetch --locked

.PHONY: back-update-deps
back-update-deps:
	@echo "Updating backend dependencies..."
	@$(COMPOSE) run --rm --no-TTY backend cargo update

.PHONY: back-install
back-install: back-lock back-sync

.PHONY: back-sh
back-sh:
	@$(COMPOSE) run -it backend bash
