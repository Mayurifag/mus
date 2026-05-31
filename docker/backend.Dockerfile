# syntax=docker/dockerfile:1.7

FROM rust:1-alpine3.22

WORKDIR /app

ENV DATA_DIR_PATH=/app_data
ENV CARGO_HOME=/cargo
ENV XDG_CACHE_HOME=/app_data/.cache
ENV HOME=/home/appuser

ARG USER_ID=1000
ARG GROUP_ID=1000
ARG TARGETARCH

RUN apk add --no-cache \
        build-base \
        ffmpeg \
        curl \
        cargo-audit \
        cargo-watch \
        python3 \
        sqlite \
        sudo \
    && curl -fsSL https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

RUN group_name="$(awk -F: -v gid="$GROUP_ID" '$3 == gid { print $1; exit }' /etc/group)" \
    && if [ -z "$group_name" ]; then addgroup -g "$GROUP_ID" appgroup && group_name=appgroup; fi \
    && adduser -D -u "$USER_ID" -G "$group_name" appuser \
    && mkdir -p $DATA_DIR_PATH/.cache/covers $DATA_DIR_PATH/music /home/appuser/.local/bin /cargo /app/target \
    && chown appuser:"$group_name" /usr/local/bin/yt-dlp \
    && chown -R appuser:"$group_name" /app $DATA_DIR_PATH /home/appuser /cargo /usr/local/cargo \
    && echo "appuser ALL=(ALL) NOPASSWD: /bin/chown -R appuser* /app/target* /cargo*" >> /etc/sudoers

RUN rustup component add rustfmt clippy

COPY backend/Cargo.toml backend/Cargo.lock ./
RUN --mount=type=cache,target=/cargo/registry \
    --mount=type=cache,target=/cargo/git \
    mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --locked \
    && rm -rf src

USER appuser

ENV PATH="/usr/local/cargo/bin:/cargo/bin:/home/appuser/.local/bin:${PATH}"

EXPOSE 8001

CMD ["sh", "-c", "sudo chown -R appuser:$(id -gn appuser) /app/target /cargo 2>/dev/null || true; cargo-watch -x 'run --locked'"]
