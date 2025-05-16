DOCKER_COMPOSE_CMD := docker compose -f docker/docker-compose.yml

.PHONY: ci
ci: back-format back-lint back-test front-format front-lint front-svelte-check front-test
