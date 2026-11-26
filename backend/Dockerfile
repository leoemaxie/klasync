# Build context: ./backend  (docker compose build uses this directory)

# --- Build stage -------------------------------------------------------------
ARG RUNTIME_IMAGE=debian:bookworm-slim
FROM rust:1-bookworm AS builder
WORKDIR /app
ENV SQLX_OFFLINE=true

# Cache dependencies: build a stub first so source edits don't rebuild the world.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && echo '' > src/lib.rs \
    && cargo build --release 2>/dev/null || true
RUN rm -rf src

COPY .sqlx ./.sqlx
COPY migrations ./migrations
COPY src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release

# --- Runtime stage -----------------------------------------------------------
# RUNTIME_IMAGE / INSTALL_RUNTIME_DEPS exist for build environments without
# access to Debian mirrors (use a base that already ships libssl3 + curl).
ARG RUNTIME_IMAGE=debian:bookworm-slim
FROM ${RUNTIME_IMAGE} AS runner
ARG INSTALL_RUNTIME_DEPS=true
RUN if [ "$INSTALL_RUNTIME_DEPS" = "true" ]; then \
      apt-get update \
      && apt-get install -y --no-install-recommends ca-certificates libssl3 curl tzdata \
      && rm -rf /var/lib/apt/lists/*; \
    fi \
    && groupadd -g 10001 klasync && useradd -u 10001 -g klasync -s /usr/sbin/nologin klasync

WORKDIR /app
COPY --from=builder /app/target/release/klasync-backend /app/klasync-backend

ENV PORT=8787 \
    HOST=0.0.0.0 \
    PUBLIC_APP_URL=http://localhost:5173 \
    CORS_ALLOWED_ORIGINS=http://localhost:5173 \
    REDIS_KEY_PREFIX=klasync:production \
    REDIS_CONNECT_TIMEOUT_MS=10000 \
    REDIS_COMMAND_TIMEOUT_MS=5000

USER klasync:klasync
EXPOSE 8787
HEALTHCHECK --interval=15s --timeout=5s --start-period=30s --retries=5 \
    CMD curl -fsS http://127.0.0.1:8787/health || exit 1
CMD ["/app/klasync-backend"]
