ARGS = $(filter-out $@,$(MAKECMDGOALS))

%:
	@:

include docker/makefiles/*.mk
