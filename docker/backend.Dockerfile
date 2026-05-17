FROM rust:1-alpine

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
        cargo-watch \
        sudo \
        yt-dlp

RUN group_name="$(awk -F: -v gid="$GROUP_ID" '$3 == gid { print $1; exit }' /etc/group)" \
    && if [ -z "$group_name" ]; then addgroup -g "$GROUP_ID" appgroup && group_name=appgroup; fi \
    && adduser -D -u "$USER_ID" -G "$group_name" appuser \
    && mkdir -p $DATA_DIR_PATH/covers $DATA_DIR_PATH/music $DATA_DIR_PATH/.cache /home/appuser/.local/bin /cargo /app/target \
    && chown -R appuser:"$group_name" /app $DATA_DIR_PATH /home/appuser /cargo /usr/local/cargo \
    && echo "appuser ALL=(ALL) NOPASSWD: /bin/chown -R appuser* /app/target* /cargo*" >> /etc/sudoers

RUN rustup component add rustfmt clippy

COPY backend-rs/Cargo.toml backend-rs/Cargo.lock ./
RUN mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --locked \
    && rm -rf src

RUN CARGO_TARGET_DIR=/tmp/cargo-install-target cargo install --locked --root /usr/local/cargo cargo-audit cargo-machete \
    && rm -rf /tmp/cargo-install-target

USER appuser

ENV PATH="/usr/local/cargo/bin:/cargo/bin:/home/appuser/.local/bin:${PATH}"

EXPOSE 8001

CMD ["sh", "-c", "sudo chown -R appuser:$(id -gn appuser) /app/target /cargo 2>/dev/null || true; cargo-watch -x 'run --locked'"]
