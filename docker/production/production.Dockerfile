FROM node:24-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/ ./
ENV VITE_INTERNAL_API_HOST=""
ENV VITE_PUBLIC_API_HOST=""
RUN npm install -g npm@11.14.1 \
    && npm ci --no-fund --prefer-offline \
    && npm run build

FROM rust:1-alpine3.22 AS backend-builder
WORKDIR /app/backend-rs
RUN apk add --no-cache build-base
COPY backend-rs/Cargo.toml backend-rs/Cargo.lock ./
RUN mkdir src && printf 'fn main() {}\n' > src/main.rs && cargo build --locked --release && rm -rf src
COPY backend-rs/src ./src
RUN cargo clean --release -p mus-backend && cargo build --locked --release

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
        su-exec \
        yt-dlp

WORKDIR /app

COPY --from=backend-builder /app/backend-rs/target/release/mus-backend /app/mus-backend
COPY --from=frontend-builder /app/frontend/build /app/frontend/build
COPY docker/production/entrypoint.sh /app/entrypoint.sh

RUN mkdir -p /app_data/covers /app_data/.cache \
    && chmod a+rx /app/mus-backend /app/entrypoint.sh

EXPOSE 8000

HEALTHCHECK --interval=300s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8000/api/healthcheck.json || exit 1

ENTRYPOINT ["/app/entrypoint.sh"]
CMD ["/app/mus-backend"]
