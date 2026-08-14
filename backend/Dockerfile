# --- Build Stage ---
FROM rust:1.85-alpine AS builder
RUN apk add --no-cache build-base musl-dev openssl-dev pkgconfig

WORKDIR /app
ENV SQLX_OFFLINE=true

# Copy root workspace manifests
COPY Cargo.toml Cargo.lock ./
# Remove desktop member so Cargo workspace resolution succeeds without src-tauri
RUN sed -i '/"src-tauri"/d' Cargo.toml

# Copy backend source, migrations, and cached sqlx query metadata
COPY backend ./backend

RUN cargo build --release --package klasync-backend

# --- Runtime Stage ---
FROM alpine:3.20 AS runner
RUN apk add --no-cache ca-certificates libgcc tzdata && \
    addgroup -g 10001 -S klasync && \
    adduser -u 10001 -S klasync -G klasync

WORKDIR /app

COPY --from=builder /app/target/release/klasync-backend /app/klasync-backend
COPY --from=builder /app/backend/migrations /app/migrations
COPY backend/.env.example /app/.env.example

# Default runtime configuration environment variables
ENV PORT=8787 \
    HOST=0.0.0.0 \
    PUBLIC_APP_URL=http://localhost:5173 \
    CORS_ALLOWED_ORIGINS=http://localhost:5173 \
    REDIS_KEY_PREFIX=klasync:production \
    REDIS_CONNECT_TIMEOUT_MS=10000 \
    REDIS_COMMAND_TIMEOUT_MS=5000

USER klasync:klasync

EXPOSE 8787

CMD ["/app/klasync-backend"]
