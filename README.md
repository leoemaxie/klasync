# KLASYNC — Assistive Learning Infrastructure for Higher Education
Codex Session ID: 019f7b7e-2d62-7272-bdb1-5dd549904ee6

> **Turn every physical lecture into an accessible, searchable, AI-powered learning experience.**

KLASYNC is an accessibility-first learning platform engineered for university auditoriums and lecture halls. It breaks down physical, sensory, and cognitive learning barriers by transforming physical speech into real-time captions, searchable transcripts, interactive flashcards, and verified attendance rosters without requiring students to create an account to join.

---

## ✦ Core User Journeys

- **Instant Student Guest Access**: Students join live lectures via a 6-character short code or QR code with zero sign-in friction.
- **Real-Time Live Captions**: High-contrast, low-latency live speech-to-text captions powered by WebSocket audio streaming.
- **Hardware & Device Fallback**: Auto-connecting wireless microphone hardware (`ESP32-S3`) with MediaDevices WebAudio fallback.
- **Roster & Attendance Verification**: Drag & drop CSV/XLSX roster validation matching student matric numbers seamlessly.
- **Persistent Student Archive**: Searchable transcript reader with font scaling, interactive AI revision flashcards, and audio replays.

---

## ✦ Built with Codex & GPT-5.6 Agentic Engineering

KLASYNC was architected and built using **Codex** and **GPT-5.6** advanced agentic coding workflows:

- **Strict Modular Constraint (< 120 LOC / File)**: Codex and GPT-5.6 enforced a strict architectural rule ensuring every single component and API module stays under 120 lines of code for maximum maintainability.
- **ORYZO Editorial Design System**: Implemented warm-dark palette tokens (`#ffedd7`, `#100904`, `#382416`, `#40372e`, `#dc5000`) and editorial typography.
- **Full-Stack Domain Typing**: Generated strongly typed Svelte 5 / TypeScript API client contracts (`/auth`, `/courses`, `/sessions`, `/captions`, `/archive`).
- **Responsive Visual System**: Designed responsive desktop visual panels with animated audio waveforms and speech-to-text tickers.

---

## ✦ Technology Stack

- **Frontend**: Svelte 5, TypeScript, Vite, Vanilla CSS Design System
- **Desktop Packaging**: Tauri (Rust Host)
- **API Client**: Fetch / WebSocket REST & streaming client
- **Package Manager**: `pnpm` exclusively

---

## ✦ Development & Local Setup

This repository uses **pnpm only**.

```powershell
# Run Vite development server
pnpm dev

# Run desktop Tauri development environment
pnpm tauri dev
```

---

## ✦ Project Architecture

```text
src/
  lib/
    api/         # Typed domain API modules (http, auth, courses, sessions, captions, archive)
    components/  # Modular UI primitives (HomeScreen, JoinScreen, LiveScreen, ArchiveScreen)
      archive/   # Interactive study tools (TranscriptViewer, FlashcardDeck, AudioPlayerPanel)
      auth/      # Authentication screens (LecturerSignIn, StudentSignIn, PasswordRecovery)
      home/      # Editorial landing sections (HeroSection, FeatureGridSection, LandingFooter)
      lecturer/  # Lecturer tools (RosterUploadPanel, MicStatusPanel, AudioLevelMeter)
  styles/        # CSS tokens, layout grids, and component elements
```

---

## ✦ License & Governance

KLASYNC is designed for higher education accessibility compliance. See [DESIGN.md](DESIGN.md) for binding visual direction rules.
