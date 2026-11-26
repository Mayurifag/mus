# syntax=docker/dockerfile:1.29.0

FROM node:26.13.1-alpine3.24 AS node-runtime

FROM rust:1.100.0-alpine3.24 AS rust-runtime

FROM golang:1.27.4-alpine3.24 AS reflex-builder
RUN go telemetry off && CGO_ENABLED=0 go install github.com/cespare/reflex@v0.3.2

FROM python:3.15.2-alpine3.24@sha256:0f9552bb28711333da16915ec146f5e6bb44a7fdac9ced15dc17c2e0206346dd

COPY --from=rust-runtime /usr/local/cargo /usr/local/cargo
COPY --from=rust-runtime /usr/local/rustup /usr/local/rustup

WORKDIR /app

ENV DATA_DIR_PATH=/app_data
ENV CARGO_HOME=/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV XDG_CACHE_HOME=/app_data/.cache
ENV HOME=/home/appuser
ENV PATH="/usr/local/cargo/bin:${PATH}"

ARG USER_ID=1000
ARG GROUP_ID=1000
ARG TARGETARCH

COPY --chmod=644 docker/yt-dlp.conf /etc/yt-dlp.conf
# Alpine's Node package can lag behind upstream security releases.
COPY --from=node-runtime /usr/local/bin/node /usr/bin/node

RUN apk add --no-cache \
        build-base \
        ffmpeg \
        curl \
        libatomic \
        libstdc++ \
        sudo \
    && curl -fsSL https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

RUN group_name="$(awk -F: -v gid="$GROUP_ID" '$3 == gid { print $1; exit }' /etc/group)" \
    && if [ -z "$group_name" ]; then addgroup -g "$GROUP_ID" appgroup && group_name=appgroup; fi \
    && adduser -D -u "$USER_ID" -G "$group_name" appuser \
    && mkdir -p $DATA_DIR_PATH/.cache/covers $DATA_DIR_PATH/music /home/appuser/.local/bin /cargo /app/target \
    && chown appuser:"$group_name" /usr/local/bin /usr/local/bin/yt-dlp \
    && chown -R appuser:"$group_name" /app $DATA_DIR_PATH /home/appuser /cargo /usr/local/cargo \
    && echo "appuser ALL=(ALL) NOPASSWD: /bin/chown -R appuser* /app/target* /cargo*" >> /etc/sudoers

RUN rustup component add rustfmt clippy \
    && XDG_CACHE_HOME=/root/.cache python3 -m pip --disable-pip-version-check uninstall --root-user-action=ignore --yes pip

COPY --from=reflex-builder /go/bin/reflex /usr/local/bin/reflex

USER appuser

ENV PATH="/usr/local/cargo/bin:/cargo/bin:/home/appuser/.local/bin:${PATH}"

EXPOSE 8001

CMD ["sh", "-c", "sudo chown -R appuser:$(id -gn appuser) /app/target /cargo 2>/dev/null || true; exec reflex -s -r '^(src/|Cargo\\.(toml|lock)$)' -R '^target/' -- cargo run --locked"]
