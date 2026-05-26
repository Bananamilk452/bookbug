# AGENTS.md

## Stack

- **Frontend**: React 19, TypeScript 5.8, Vite 7
- **Backend**: Tauri v2, Rust (edition 2021)
- **Package manager**: pnpm (pnpm-workspace.yaml present)

## Commands

| Command            | Does                                                                                 |
| ------------------ | ------------------------------------------------------------------------------------ |
| `pnpm dev`         | Vite dev server only (port 1420). Frontend-only; does **not** start the Tauri shell. |
| `pnpm tauri dev`   | Full Tauri app: starts Vite dev server, then opens the native window.                |
| `pnpm tauri build` | Production build (runs `pnpm build` first, then `cargo build`).                      |
| `pnpm build`       | `tsc && vite build`. Typecheck first, then bundle for Tauri to consume.              |
| `cargo test`       | Rust tests (run inside `src-tauri/` or with `--manifest-path`).                      |

There is no lint, format, or test command defined for the frontend yet.

## Architecture

```
src/                   → React frontend (Vite entry: index.html → main.tsx → App.tsx)
src-tauri/src/         → Rust backend
  lib.rs                  Tauri builder + commands (crate name: `bookbug_lib`)
  main.rs                 Binary entry point
src-tauri/capabilities/ → Tauri v2 permission grants
src-tauri/tauri.conf.json → Tauri app config
```

- Frontend calls Rust commands via `invoke()` from `@tauri-apps/api/core`.
- Rust commands are registered in `lib.rs` with `tauri::generate_handler![]`.
- The Rust crate is named `bookbug_lib` (not `bookbug`) — necessary for `cargo` invocations targeting the lib (e.g. `cargo test -p bookbug_lib`).

## Tauri v2 specifics

- Permissions are capability-based (`src-tauri/capabilities/default.json`). Add new plugin permissions there, not in Cargo.toml or tauri.conf.json.
- `beforeDevCommand` is `pnpm dev`, so Vite must be ready before Tauri opens. Port 1420 is fixed (`strictPort: true`).
- The `tauri-plugin-opener` plugin is already wired up in both the Rust side and the capability file.
- Generated code lives in `src-tauri/gen/` — never edit it.

## Intent

This project is intended to become an EPUB library manager and reader. The current codebase is a Tauri + React starter template serving as the foundation.
