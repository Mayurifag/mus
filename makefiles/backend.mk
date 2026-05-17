DOCKER_COMPOSE_BACKEND_CMD := $(DOCKER_COMPOSE_CMD) run --rm --no-TTY backend
CARGO_INSTALL := CARGO_TARGET_DIR=/tmp/cargo-install-target cargo install --locked

.PHONY: back-ci
back-ci: back-format-check back-check back-lint back-test back-machete back-audit

.PHONY: back-lint
back-lint:
	@echo "Linting backend code..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) cargo clippy --locked --all-targets --all-features -- -D warnings

.PHONY: back-check
back-check:
	@echo "Checking backend code..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) cargo check --locked --all-targets --all-features

.PHONY: back-format
back-format:
	@echo "Formatting backend code..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) cargo fmt

.PHONY: back-format-check
back-format-check:
	@echo "Checking backend code formatting..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) cargo fmt -- --check

.PHONY: back-test
back-test:
	@echo "Running backend tests..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) cargo test --locked --all-targets --all-features

.PHONY: back-machete
back-machete:
	@echo "Checking backend dependencies..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) sh -c 'test -x /cargo/bin/cargo-machete || $(CARGO_INSTALL) cargo-machete'
	@$(DOCKER_COMPOSE_BACKEND_CMD) /cargo/bin/cargo-machete

.PHONY: back-audit
back-audit:
	@echo "Auditing backend dependencies..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) sh -c 'test -x /cargo/bin/cargo-audit || $(CARGO_INSTALL) cargo-audit'
	@$(DOCKER_COMPOSE_BACKEND_CMD) /cargo/bin/cargo-audit audit --deny warnings

.PHONY: back-clean
back-clean:
	@echo "Removing backend build artifacts..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) cargo clean

.PHONY: back-lock
back-lock:
	@echo "Locking backend dependencies..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) cargo generate-lockfile

.PHONY: back-sync
back-sync:
	@echo "Fetching backend dependencies..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) cargo fetch --locked

.PHONY: back-update-deps
back-update-deps:
	@echo "Updating backend dependencies..."
	@$(DOCKER_COMPOSE_BACKEND_CMD) cargo update

.PHONY: back-install
back-install: back-lock back-sync

.PHONY: back-sh
back-sh:
	@$(DOCKER_COMPOSE_CMD) run -it backend bash
