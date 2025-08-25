ARGS = $(filter-out $@,$(MAKECMDGOALS))

DOCKER_COMPOSE_CMD := docker compose -f docker/docker-compose.yml -f docker/docker-compose.override.yml

%:
	@:

include makefiles/*.mk
