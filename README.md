# Klasync

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Svelte 5](https://img.shields.io/badge/Svelte-5-FF3E00?logo=svelte&logoColor=white)](https://svelte.dev)
[![Rust](https://img.shields.io/badge/Rust-Axum-DEA584?logo=rust&logoColor=black)](https://www.rust-lang.org/)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app)
[![pnpm](https://img.shields.io/badge/maintained%20with-pnpm-f69220?logo=pnpm&logoColor=white)](https://pnpm.io)

> Accessibility-first lecture platform for higher education.

Klasync transforms in-person university lectures into live-captioned, searchable, and structured learning sessions. Built on a guest-first model, students can join any active lecture instantly via a 6-character code or QR scan—without creating an account.

---

## Features

- **Guest-First Access** — Students join live sessions instantly using a short code or QR scan. No sign-up friction.
- **Real-Time Captions** — Low-latency, high-contrast speech-to-text streaming directly to student devices.
- **Roster & Attendance** — Automatic matriculation number validation against uploaded course rosters (CSV/XLSX).
- **Interactive Archive** — Searchable transcripts, lecture key points, and AI-generated revision flashcards.
- **Hardware & Web Fallback** — Dedicated wireless microphone support with automatic browser audio fallback.
- **Cross-Platform** — Lightweight web application wrapped in a native desktop host.

---

## Tech Stack

| Layer | Technologies |
|---|---|
| **Frontend** | Svelte 5, TypeScript, Vite |
| **Desktop** | Tauri v2 (Rust host) |
| **Backend** | Rust (Axum), PostgreSQL, Redis |
| **Storage & Sync** | IndexedDB (offline cache), WebSockets & REST streaming |
| **Package Manager** | `pnpm` (strictly enforced for frontend) |

---

## Project Structure

```text
klasync/
├── src/          # Frontend web application (Svelte 5, TypeScript, Vite)
├── backend/      # Backend API service (Rust, Axum, PostgreSQL, Redis)
├── src-tauri/    # Desktop application host (Tauri v2)
└── static/       # Static web assets, branding, and templates
```

---

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18 or higher)
- [pnpm](https://pnpm.io/) (`corepack enable pnpm` or `npm i -g pnpm`)
- [Rust](https://www.rust-lang.org/) (for Tauri desktop client & backend service)

### Frontend & Desktop Development

```bash
# Clone the repository
git clone https://github.com/leoemaxie/klasync.git
cd klasync

# Install frontend dependencies
pnpm install

# Start the web client (Vite)
pnpm dev

# Start the desktop application (Tauri + Vite)
pnpm tauri dev
```

### Backend Service

The standalone Axum API server is located in the [`backend/`](backend/) directory. For complete setup, database migrations, and configuration details, see [backend/README.md](backend/README.md).

```bash
# Navigate to the backend directory
cd backend

# Configure environment variables
cp .env.example .env

# Run database migrations (requires sqlx-cli)
cargo sqlx migrate run

# Start the backend server
cargo run
```

### Verification & Builds

```bash
# Type check and Svelte diagnostics
pnpm check

# Production web build
pnpm build

# Format source files
pnpm format
```

---

## License

MIT
