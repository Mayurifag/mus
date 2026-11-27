# syntax=docker/dockerfile:1.29.0

FROM node:26.13.1-alpine3.24 AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
ENV VITE_INTERNAL_API_HOST=""
ENV VITE_PUBLIC_API_HOST=""
RUN --mount=type=cache,target=/root/.npm npm ci --no-fund --prefer-offline
COPY frontend/ ./
RUN npm run build

FROM rust:1.100.0-alpine3.24 AS backend-builder
WORKDIR /app/backend
RUN apk add --no-cache build-base
COPY backend/Cargo.toml backend/Cargo.lock ./
# Export a regular layer so fresh CI builders can reuse compiled dependencies.
RUN mkdir src \
    && touch src/lib.rs \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --locked --release \
    && cargo clean --release -p mus-backend \
    && rm -r src
COPY backend/src ./src
RUN cargo build --locked --release --offline

FROM python:3.15.2-alpine3.24@sha256:0f9552bb28711333da16915ec146f5e6bb44a7fdac9ced15dc17c2e0206346dd AS runtime-base
# Refresh Alpine packages regularly without reinstalling FFmpeg for every app change.
ARG APK_CACHE_EPOCH
RUN apk add --no-cache \
        curl \
        ffmpeg \
        libatomic \
        libstdc++ \
        su-exec \
    && mkdir -p /app_data/.cache/covers \
    && python3 -m pip --disable-pip-version-check uninstall --root-user-action=ignore --yes pip

FROM runtime-base AS downloader
# Each published build fetches the current nightly independently of Alpine packages.
ARG BUILD_DATE
RUN curl -fsSL https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp

FROM runtime-base
ENV LOG_LEVEL=info \
    PORT=8000 \
    DATA_DIR_PATH=/app_data \
    STATIC_DIR_PATH=/app/frontend/build \
    XDG_CACHE_HOME=/app_data/.cache \
    HOME=/root

COPY --chmod=644 docker/yt-dlp.conf /etc/yt-dlp.conf
# Alpine's Node package can lag behind upstream security releases.
COPY --from=frontend-builder /usr/local/bin/node /usr/bin/node

WORKDIR /app
COPY --link --chmod=755 --from=downloader /usr/local/bin/yt-dlp /usr/local/bin/yt-dlp
COPY --link --chmod=755 --from=backend-builder /app/backend/target/release/mus-backend /app/mus-backend
COPY --link --from=frontend-builder /app/frontend/build /app/frontend/build
COPY --link --chmod=755 docker/production/entrypoint.sh /app/entrypoint.sh

ARG BUILD_DATE
ARG COMMIT_SHA
ARG COMMIT_TITLE
LABEL org.opencontainers.image.created=$BUILD_DATE \
      org.opencontainers.image.revision=$COMMIT_SHA \
      org.opencontainers.image.title=$COMMIT_TITLE
ENV COMMIT_SHA=$COMMIT_SHA BUILD_DATE=$BUILD_DATE

EXPOSE 8000

HEALTHCHECK --interval=300s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8000/api/healthcheck.json || exit 1

ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["/app/mus-backend"]
