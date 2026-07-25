# --- Build Stage ---
FROM rust:1.85-alpine AS builder
RUN apk add --no-core build-base musl-dev openssl-dev pkgconfig

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend

RUN cargo build --release --package klasync-backend

# --- Runtime Stage ---
FROM alpine:3.20 AS runner
RUN apk add --no-cache ca-certificates libgcc

WORKDIR /app
COPY --from=builder /app/target/release/klasync-backend /app/klasync-backend
COPY --from=builder /app/backend/migrations /app/migrations

ENV PORT=8787
ENV HOST=0.0.0.0

EXPOSE 8787

CMD ["/app/klasync-backend"]
