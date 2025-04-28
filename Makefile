ARGS := $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS))
$(eval $(ARGS):;@:)

.PHONY: all build run test lint format clean

all: build

include docker/makefiles/build.mk
include docker/makefiles/deps.mk
include docker/makefiles/run.mk
include docker/makefiles/test.mk
include docker/makefiles/lint.mk
include docker/makefiles/format.mk
include docker/makefiles/clean.mk
