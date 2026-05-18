# syntax=docker/dockerfile:1.7

FROM node:24-alpine3.22 AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
ENV VITE_INTERNAL_API_HOST=""
ENV VITE_PUBLIC_API_HOST=""
RUN --mount=type=cache,target=/root/.npm \
    npm install -g npm@11.14.1 \
    && npm ci --no-fund --prefer-offline
COPY frontend/ ./
RUN npm run build

FROM rust:1-alpine3.22 AS backend-builder
WORKDIR /app/backend
RUN apk add --no-cache build-base
COPY backend/Cargo.toml backend/Cargo.lock ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/backend/target \
    mkdir src && printf 'fn main() {}\n' > src/main.rs && cargo build --locked --release && rm -rf src
COPY backend/src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/backend/target \
    cargo clean --release -p mus-backend \
    && cargo build --locked --release \
    && cp target/release/mus-backend /tmp/mus-backend

FROM alpine:3.22
ARG TARGETARCH
ARG BUILD_DATE
ARG COMMIT_SHA
ARG COMMIT_TITLE

ENV APP_ENV=production \
    LOG_LEVEL=info \
    PORT=8000 \
    DATA_DIR_PATH=/app_data \
    STATIC_DIR_PATH=/app/frontend/build \
    XDG_CACHE_HOME=/app_data/.cache \
    HOME=/root

LABEL org.opencontainers.image.created=$BUILD_DATE
LABEL org.opencontainers.image.revision=$COMMIT_SHA
LABEL org.opencontainers.image.title=$COMMIT_TITLE

ENV COMMIT_SHA=$COMMIT_SHA
ENV BUILD_DATE=$BUILD_DATE

RUN apk add --no-cache \
        curl \
        ffmpeg \
        python3 \
        sqlite \
        su-exec \
    && curl -fsSL https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

WORKDIR /app

COPY --from=backend-builder /tmp/mus-backend /app/mus-backend
COPY --from=frontend-builder /app/frontend/build /app/frontend/build
COPY docker/production/entrypoint.sh /app/entrypoint.sh

RUN mkdir -p /app_data/covers /app_data/.cache \
    && chmod a+rx /app/mus-backend /app/entrypoint.sh

EXPOSE 8000

HEALTHCHECK --interval=300s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8000/api/healthcheck.json || exit 1

ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["/app/mus-backend"]
