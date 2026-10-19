.PHONY: test-prod
test-prod: test-prod-stop
	@echo "Testing production Docker image..."
	@echo "Building production image..."
	@docker build -f docker/production/production.Dockerfile -t mus:test .
	@echo "Starting production container with music folder from override config..."
	@MUSIC_PATH=$$(grep -A 15 "backend:" docker/docker-compose.override.yml | grep "/app_data/music" | sed 's/.*- \([^:]*\):.*/\1/' | xargs) && \
	echo "Using music path: $$MUSIC_PATH" && \
	docker run -d --name mus-prod-test -p 127.0.0.1:4124:8000 \
		--security-opt no-new-privileges:true \
		-e SECRET_KEY=test-secret-key-123 \
		-v "$$MUSIC_PATH":/app_data/music \
		mus:test
	@echo "Waiting for container to start..."
	@echo "Production container started at http://localhost:4124"
	@echo "Authentication enabled with SECRET_KEY=test-secret-key-123"
	@echo "Magic link: http://localhost:4124/login?token=test-secret-key-123"
	@echo "Use 'make test-prod-stop' to stop and clean up"

.PHONY: test-prod-stop
test-prod-stop:
	@echo "Stopping and removing production test container..."
	@docker stop mus-prod-test 2>/dev/null || true
	@docker rm mus-prod-test 2>/dev/null || true
	@docker rmi mus:test 2>/dev/null || true
	@echo "Production test cleanup complete"

.PHONY: prod-downloader-smoke
prod-downloader-smoke:
	@docker run --rm --network none --user 65534:65534 --entrypoint python3 mus:latest -c 'import sys, subprocess; sys.path.insert(0, "/usr/local/bin/yt-dlp"); import yt_dlp, yt_dlp_ejs; from yt_dlp_ejs.yt import solver; subprocess.run(["node", "--version"], check=True); subprocess.run(["yt-dlp", "--version"], check=True); opts = yt_dlp.parse_options().ydl_opts; assert set(opts["js_runtimes"]) == {"node"}, "System config must enable Node"; assert not opts["remote_components"]; assert yt_dlp.YoutubeDL(opts)._js_runtimes["node"].info.supported; assert solver.lib() and solver.core(); print("Offline downloader runtime checks passed; bundled EJS:", yt_dlp_ejs.version)'

.PHONY: prod-smoke
prod-smoke: prod-downloader-smoke
	@docker image inspect mus:latest >/dev/null
	@docker rm -f mus-smoke-test 2>/dev/null || true
	@MUSIC_TMP=$$(mktemp -d); \
	trap 'docker rm -f mus-smoke-test >/dev/null 2>&1 || true; rm -rf "$$MUSIC_TMP"' EXIT; \
	cp -R e2e/music/. "$$MUSIC_TMP"; \
	docker run -d --name mus-smoke-test -p 127.0.0.1:4125:8000 \
		--security-opt no-new-privileges:true \
		-v "$$MUSIC_TMP:/app_data/music:rw" \
		mus:latest >/dev/null; \
	for i in 1 2 3 4 5 6 7 8 9 10; do \
		curl -fsS "http://localhost:4125/api/healthcheck.json" >/dev/null 2>&1 && break; \
		sleep 2; \
		if [ "$$i" = "10" ]; then docker logs mus-smoke-test; exit 1; fi; \
	done; \
	curl -fsS "http://localhost:4125/api/healthcheck.json" >/dev/null; \
	curl -fsSI "http://localhost:4125/" >/dev/null; \
	curl -fsSI "http://localhost:4125/some/spa/route" >/dev/null; \
	curl -fsS "http://localhost:4125/api/v1/tracks" >/dev/null; \
	curl -fsSI "http://localhost:4125/api/v1/events/track-updates" | grep -qi '^content-type: text/event-stream'; \
	echo "Production smoke checks passed"

.PHONY: prod-verify
prod-verify:
	@$(MAKE) prod-image
	@$(MAKE) prod-smoke
	@$(MAKE) e2e-current-image
