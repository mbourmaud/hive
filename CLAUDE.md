# HIVE Project Instructions

## Overview

Rust CLI + Desktop App for orchestrating multiple Claude Code instances (drones) via git worktrees. Features a TUI dashboard, web-based monitor, and native desktop app with an OpenCode-style chat interface.

## Commands

```bash
hive init                              # Initialize Hive in current repo
hive start <name>                      # Launch a drone with a plan
hive monitor                           # TUI dashboard for all drones
hive logs <name>                       # View drone activity log
hive stop <name>                       # Stop a running drone
hive clean <name>                      # Remove drone & worktree
hive list                              # Quick list of all drones
hive update                            # Self-update to latest version
```

## Dependencies

- git, gh CLI (for PR operations), claude CLI
- Rust toolchain (1.75+), Node.js 20+
- For desktop: Tauri v2 system dependencies

## Project Structure

```
src/                    # Rust source code
  webui/                # Axum web server
    mod.rs              # Routes, SSE, drone monitoring API
    chat.rs             # Chat session backend (Claude CLI subprocess + SSE)
  commands/             # CLI command implementations
  agent_teams/          # Drone orchestration logic
  plan_parser.rs        # Markdown plan parser
  config.rs             # Configuration management
  events.rs             # Hook-based event streaming

web/                    # React frontend (Vite + Tailwind v4)
  src/
    components/
      chat/             # Chat UI (OpenCode-inspired)
        session-turn.tsx    # Turn renderer with steps collapse
        chat-layout.tsx     # Message list + progressive render + auto-scroll
        prompt-input.tsx    # Auto-resize textarea with history
        markdown-renderer.tsx  # marked + DOMPurify + morphdom pipeline
        code-block.tsx      # Shiki syntax highlighting
        diff-viewer.tsx     # Unified diff renderer
        basic-tool.tsx      # Collapsible tool wrapper (Radix)
        tool-registry.ts    # Pluggable tool renderers
        session-list.tsx    # Session history (grouped by date)
        parts/              # 11 specialized tool renderers
      layout/             # App layout components
        app-sidebar.tsx     # Resizable dual-mode sidebar
        mode-switcher.tsx   # Chat/Monitor toggle
    hooks/
      use-chat.ts           # SSE streaming + RAF event coalescing
      use-chat-reducer.ts   # Event-sourced state machine
      use-sessions.ts       # Session list + date grouping
      use-projects.ts       # SSE drone monitoring
    types/
      chat.ts               # Chat type system (discriminated unions)
      api.ts                # Monitor API types

desktop/                # Tauri v2 native desktop shell
  src-tauri/
    src/main.rs         # Plugin registration, window management
    Cargo.toml          # Tauri plugins
    capabilities/       # Permission grants

.hive/                  # Created by 'hive init' (runtime data)
  config.json           # Configuration
  plans/                # Markdown plan files
  drones/               # Drone status and logs
  sessions/             # Chat session persistence (events.ndjson + meta.json)

tests/                  # Rust test suite
```

---

## Code Standards

### Rust

- **Error handling**: Use `anyhow::Result` for application errors, `thiserror` for library errors. Avoid manual `StatusCode` + JSON error responses — prefer Axum's `IntoResponse` trait implementations.
- **Async I/O**: Use `tokio::fs` for file operations in async contexts. `std::fs` is acceptable only in synchronous helper functions called outside the async runtime.
- **Locking**: Use `tokio::sync::Mutex` for state shared across `.await` points. Use `std::sync::Mutex` only for synchronous-only access. Prefer `RwLock` when reads outnumber writes.
- **Process management**: Always handle `Child` process cleanup. Use `tokio::select!` for timeout + process wait patterns. Send SIGTERM before SIGKILL.
- **Serde**: Derive `Serialize`/`Deserialize` with `#[serde(rename_all = "snake_case")]`. Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields.
- **Clippy**: Run `cargo clippy -- -W clippy::all` before committing. Fix all warnings.
- **Tests**: Use `#[tokio::test]` for async tests. Avoid `set_current_dir` in tests (causes race conditions).

### TypeScript / React

- **NEVER use `any`** — use `unknown`, proper types, or `Record<string, unknown>`.
- **Discriminated unions**: Use `type` field for union discrimination (see `StreamEvent`, `AssistantPart`).
- **Hooks**: Follow rules of hooks strictly. Only wrap in `useCallback`/`useMemo` when there's a measurable perf benefit or a dependency requires stable references.
- **Components**: Use `data-component` and `data-slot` CSS selectors (not className-based styling) for component structure. Keep components focused — extract when >200 lines.
- **Events**: Always clean up event listeners, timers, observers in `useEffect` return functions.
- **Keys**: Use stable, unique IDs for list keys — never array indices.

### CSS / Tailwind

- **Design tokens**: Use OKLCH color space for all custom properties. Define in `:root` and `[data-theme="dark"]`.
- **Selectors**: Prefer `[data-component="name"]` and `[data-slot="name"]` over class-based selectors for component CSS.
- **Animations**: Only animate `transform` and `opacity` for GPU compositing. Use CSS transitions for simple state changes, `@keyframes` for complex sequences.
- **Responsive**: Mobile-first. Use `sm:` breakpoint (640px) for desktop overrides.
- **Font stack**: Mono: `'IBM Plex Mono', 'JetBrains Mono', 'SF Mono', 'Fira Code', monospace`. Sans: `'Inter', system-ui, sans-serif`.

### Tauri v2

- **Capabilities**: Follow principle of least privilege — only grant permissions actually used.
- **CSP**: Tighten CSP in production (currently `null` for dev).
- **IPC**: Use Tauri commands for Rust↔JS communication. Avoid `window.__TAURI__` globals.
- **Plugins**: Register plugins before `.setup()` in main.rs.

---

## Key Patterns

### Chat Architecture
```
Claude CLI (--output-format stream-json)
  → Rust: stdout BufReader → broadcast::channel per session
    → Axum SSE endpoint (30s heartbeat)
      → Frontend: EventSource → RAF queue (16ms batching)
        → useReducer (event-sourced state)
          → React components
```

### Event Coalescing
Events from Claude stream at high frequency. We batch them:
1. SSE events enqueue into a `queueRef`
2. `requestAnimationFrame` flushes every ~16ms
3. `coalesceEvents()` merges consecutive text-only events
4. Single `STREAM_EVENT_BATCH` dispatch to reducer

### Tool Registry
Tools register via side-effect imports. Each tool renderer wraps `BasicTool` (Radix Collapsible). Unknown tools fall back to `GenericTool`.

### Session Persistence
- Events: `.hive/sessions/{id}/events.ndjson` (append-only)
- Metadata: `.hive/sessions/{id}/meta.json`
- Replay: `REPLAY_HISTORY` reducer action reconstructs turns from events

---

## Development

```bash
# Rust
cargo check                    # Type check
cargo clippy                   # Lint
cargo test                     # Run tests
cargo build --release          # Release build

# Web UI
cd web
npm install                    # Install deps
npx vite dev                   # Dev server (hot reload)
npx tsc --noEmit               # Type check
npx vite build                 # Production build (single-file HTML)

# Desktop
cd desktop
npm install                    # Install JS deps
cargo tauri dev                # Dev mode with hot reload
cargo tauri build              # Production build

# Full check
cargo check && cd web && npx tsc --noEmit && npx vite build
```

## Installation

```bash
curl -fsSL https://raw.githubusercontent.com/mbourmaud/hive/main/install.sh | bash
```
