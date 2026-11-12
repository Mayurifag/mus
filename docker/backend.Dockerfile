# syntax=docker/dockerfile:1.29.0

FROM node:26.13.1-alpine3.24 AS node-runtime

FROM rust:1.99.1-alpine3.24 AS rust-runtime

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

COPY docker/yt-dlp.conf /etc/yt-dlp.conf
# Alpine's Node package can lag behind upstream security releases.
COPY --from=node-runtime /usr/local/bin/node /usr/bin/node

RUN apk add --no-cache \
        build-base \
        ffmpeg \
        curl \
        libatomic \
        libstdc++ \
        sqlite \
        sudo \
    && curl -fsSL https://github.com/yt-dlp/yt-dlp-nightly-builds/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp \
    && chmod a+rx /usr/local/bin/yt-dlp

# Patch the published lockfile for RUSTSEC-2026-0486 without changing its other package versions.
RUN --mount=type=cache,target=/cargo/registry \
    --mount=type=cache,target=/cargo/git \
    curl -fsSL https://static.crates.io/crates/cargo-audit/cargo-audit-0.23.0.crate -o /tmp/cargo-audit.crate \
    && echo "956f85aedfb64db887ec38466f982fc2b1ae55de726d4a12f2fddd5b812c6446  /tmp/cargo-audit.crate" | sha256sum -c - \
    && mkdir /tmp/cargo-audit \
    && tar -xzf /tmp/cargo-audit.crate -C /tmp/cargo-audit --strip-components=1 \
    && cargo update --manifest-path /tmp/cargo-audit/Cargo.toml --package rustix@1.1.5 --precise 1.1.6 \
    && echo "1210c95d880201936e1c7aab83b8e0da981cec37ae6d551dcc9668d62ee5b12b  /tmp/cargo-audit/Cargo.lock" | sha256sum -c - \
    && cargo install --path /tmp/cargo-audit --locked --root /usr/local/cargo \
    && rm -rf /tmp/cargo-audit /tmp/cargo-audit.crate

# Update the published lockfile to remove RustSec advisories from the watcher.
RUN --mount=type=cache,target=/cargo/registry \
    --mount=type=cache,target=/cargo/git \
    curl -fsSL https://static.crates.io/crates/watchexec-cli/watchexec-cli-2.8.0.crate -o /tmp/watchexec.crate \
    && echo "a9e73398f1d43104592c8b3aed966fac990773962dc3fd11939ba4cee1d23745  /tmp/watchexec.crate" | sha256sum -c - \
    && mkdir /tmp/watchexec \
    && tar -xzf /tmp/watchexec.crate -C /tmp/watchexec --strip-components=1 \
    && cargo update --manifest-path /tmp/watchexec/Cargo.toml --package notify-rust@4.18.1 --precise 4.19.0 \
    && cargo update --manifest-path /tmp/watchexec/Cargo.toml --package tokio@1.53.2 --precise 1.54.1 \
    && cargo update --manifest-path /tmp/watchexec/Cargo.toml --package rustix@1.1.5 --precise 1.1.6 \
    && echo "9dddaa99852ae71faaf4c1ac1f61ee9886567fbb1815c81051e4a367753ed675  /tmp/watchexec/Cargo.lock" | sha256sum -c - \
    && cargo install --path /tmp/watchexec --locked --root /usr/local/cargo \
    && rm -rf /tmp/watchexec /tmp/watchexec.crate

RUN group_name="$(awk -F: -v gid="$GROUP_ID" '$3 == gid { print $1; exit }' /etc/group)" \
    && if [ -z "$group_name" ]; then addgroup -g "$GROUP_ID" appgroup && group_name=appgroup; fi \
    && adduser -D -u "$USER_ID" -G "$group_name" appuser \
    && mkdir -p $DATA_DIR_PATH/.cache/covers $DATA_DIR_PATH/music /home/appuser/.local/bin /cargo /app/target \
    && chown appuser:"$group_name" /usr/local/bin /usr/local/bin/yt-dlp \
    && chown -R appuser:"$group_name" /app $DATA_DIR_PATH /home/appuser /cargo /usr/local/cargo \
    && echo "appuser ALL=(ALL) NOPASSWD: /bin/chown -R appuser* /app/target* /cargo*" >> /etc/sudoers

RUN rustup component add rustfmt clippy \
    && python3 -m pip --disable-pip-version-check uninstall --yes pip

COPY backend/Cargo.toml backend/Cargo.lock ./
RUN --mount=type=cache,target=/cargo/registry \
    --mount=type=cache,target=/cargo/git \
    mkdir src \
    && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --locked \
    && rm -rf src \
    && chown -R appuser:"$(id -gn appuser)" /cargo

USER appuser

ENV PATH="/usr/local/cargo/bin:/cargo/bin:/home/appuser/.local/bin:${PATH}"

EXPOSE 8001

CMD ["sh", "-c", "sudo chown -R appuser:$(id -gn appuser) /app/target /cargo 2>/dev/null || true; exec watchexec --restart --debounce 500ms --shell=none --watch src --watch-non-recursive . --filter 'src/**' --filter Cargo.toml --filter Cargo.lock -- cargo run --locked"]
