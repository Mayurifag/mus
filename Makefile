ARGS = $(filter-out $@,$(MAKECMDGOALS))

COMPOSE := USER_ID=$$(id -u) GROUP_ID=$$(id -g) docker compose -f docker/docker-compose.yml -f docker/docker-compose.override.yml

%:
	@:

include makefiles/*.mk
