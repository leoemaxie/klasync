# Klasync Backend API

> Standalone Axum API server for session orchestration, authentication, live captions, and persistence.

The Klasync backend is built with Rust and Axum. It handles lecturer authentication, course rosters, live session coordination, WebSocket streaming for captions, and PostgreSQL storage.

---

## Tech Stack

- **Framework**: [Axum](https://github.com/tokio-rs/axum) (Tokio)
- **Database**: PostgreSQL with [SQLx](https://github.com/launchbadge/sqlx)
- **Cache & Presence**: Redis
- **LLM Provider**: OpenRouter 
- **Object Storage**: AWS S3 / Cloudflare R2 compatible

---

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/) (2021 edition)
- [PostgreSQL](https://www.postgresql.org/)
- [Redis](https://redis.io/)

### Setup & Run

1. **Configure environment variables:**
   ```bash
   cp .env.example .env
   # Edit .env with your local PostgreSQL and Redis connection strings
   ```

2. **Run migrations:**
   ```bash
   # If sqlx-cli is installed
   cargo sqlx migrate run
   ```

3. **Start the development server:**
   ```bash
   cargo run
   ```

The server listens on `http://127.0.0.1:8787` by default.
