.PHONY: e2e-test-headless
e2e-test-headless:
	@./e2e/run-tests.sh headless

.PHONY: e2e-test-headed
e2e-test-headed:
	@./e2e/run-tests.sh headed

.PHONY: e2e-test-debug
e2e-test-debug:
	@./e2e/run-tests.sh debug

.PHONY: e2e-clean
e2e-clean:
	@./e2e/cleanup.sh
