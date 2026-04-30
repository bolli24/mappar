# ANGENT.md

This file provides guidance to AI agents when working with code in this repository.

## Project Overview

**mappar** is a cross-platform OpenStreetMap editor built with Rust and egui, targeting both native desktop (via
eframe/wgpu) and web browsers (via WebAssembly/Trunk).

## Build & Development Commands

```bash
# Native
cargo build                          # Build desktop app
cargo run                            # Run desktop app
cargo run -- --no-cache              # Run without caching

# WebAssembly
trunk serve                          # Dev server on port 8081
trunk build --release                # Production WASM build

# Testing
cargo test                           # Run all tests
cargo test -p mappar-css             # Run only MapCSS crate tests
cargo test <test_name>               # Run a single test

# Linting & Formatting
cargo clippy                         # Lint
cargo fmt                            # Format (hard tabs, 120 max width)
cargo check                          # Fast compilation check

# MapCSS validation CLI
cargo run --bin mapcss-validate -- path/to/file.mapcss
```

A `.env` file must exist in the project root (can be empty) for the build to succeed (`dotenvy_macro`).

## Workspace Structure

Three crates in a Cargo workspace:

- **`mappar`** (root) — Main application: API client, map data model, UI, imagery
- **`mappar-macros`** (`src/macros/`) — Proc macros: `#[derive(Memory)]` (persisted egui state),
  `#[derive(TempMemory)]` (transient egui state), `#[derive(Id)]` (integer newtype with Display/FromStr)
- **`mappar-css`** (`css/`) — MapCSS parser using `chumsky`/`logos` with `ariadne` error reporting

## Architecture

### Module Layout (`src/`)

- **`api/`** — OSM API client with OAuth2 PKCE auth. Supports production and dev servers. Uses keyring on native, web
  storage on WASM.
- **`map/`** — Core data model:
    - `map_data.rs` — `MapData` container with undo/redo (`EditHistory` via `undo` crate)
    - `element.rs` — OSM primitives: `Node`, `Way`, `Relation` wrapped in `MapElement`. Identified by
      `ElementId(i64, ElementType)`.
    - `edit.rs` — `Edit` actions (tag CRUD, node moves, element creation, way/relation modifications) applied to
      `MapData`
    - `references_index.rs` — Bidirectional index tracking which elements reference others
- **`ui/`** — egui-based interface:
    - `app.rs` — `MyApp` main state, implements `eframe::App`
    - `map_widget/` — Map rendering & editing:
        - `map_view.rs` — `MapView` reusable widget: handles base map rendering (tiles, elements, zoom/pan). Can be
          used standalone as a read-only view (implements `Widget`) or composed into `MapWidget` for interactive
          editing.
        - `mod.rs` — `MapWidget` composes `MapView` and adds interactivity: hover detection, `statig` state machine
          (`MapStateMachine`) with `EditMode::Select` and `EditMode::DrawPen`, selection highlighting, and selection
          box.
    - `map_widget/css/` — MapCSS style evaluation and caching for rendering
    - `key_value_table.rs` — Tag editor
    - `relations_table.rs` — Relation member editor
    - `input.rs` — Keybinding system
- **`imagery/`** — Tile imagery fetching from JOSM imagery sources. Platform-split caching: `local_cache.rs` (native
  filesystem) / `web_cache.rs` (IndexedDB). Web uses `gloo-worker` for image loading.
- **`util/`** — Helpers: `spawn_task!` (abstracts tokio::spawn vs wasm_bindgen_futures), `message_*!` macros for in-app
  notifications + logging, coordinate conversion utilities.

### Platform Conditional Compilation

The codebase uses `#[cfg(target_arch = "wasm32")]` / `#[cfg(not(...))]` extensively. Key differences:

- **Async runtime**: Multi-threaded tokio (native) vs single-threaded tokio (WASM)
- **Graphics backends**: Vulkan/DX12/Metal/GLES (native) vs WebGPU/WebGL (WASM)
- **Caching**: Filesystem (native) vs IndexedDB (WASM)
- **Auth callback**: Local TCP server (native) vs redirect URL (WASM)

Use `spawn_task!` macro instead of directly calling `tokio::spawn` or `wasm_bindgen_futures::spawn_local`.

### Key Patterns

- **State machines**: Edit mode interactions use `statig` compile-time state machines
- **Undo/redo**: All map edits go through the `Edit` + `Action` system, applied via `MapData` methods that record to
  `EditHistory`
- **String interning**: `lasso::ThreadedRodeo` for efficient repeated string storage (tag keys/values)
- **Hash maps**: `fxhash` is used for performance-sensitive hash maps; `util::HashMapExt`/`HashSetExt` traits provide
  `with_capacity` for hasher-generic construction

## Version Control

This project uses **jujutsu (`jj`)** for version control, with a git backend (`.jj/` directory). Use `jj` commands
instead of `git`:

```bash
jj status                            # Working copy status
jj log                               # Commit log
jj diff                              # Show changes
jj new                               # Start a new change
jj commit -m "message"               # Commit current change
jj describe -m "message"             # Set description of current change
```

The underlying git repo is managed by jujutsu — avoid using raw `git` commands directly.

## Code Style

- Rust edition 2024
- Hard tabs, max 120 chars per line (`rustfmt.toml`)
- `#![warn(clippy::all, rust_2018_idioms)]` enabled
- Use `Result`/`Option` properly; `thiserror` for typed errors, `anyhow` for ad-hoc errors
- Profiling: `puffin` scopes are used throughout; profiler server on `127.0.0.1:8585` in debug builds
- Always run `cargo fmt` before finishing.