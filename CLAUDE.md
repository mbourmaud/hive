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

## Design Principles

### File size limit — 300 lines max

**No file may exceed 300 lines of code.** This is a hard limit for both Rust (`.rs`) and TypeScript/React (`.ts`, `.tsx`). If a file approaches this limit, split it before it grows further. Strategies:
- **Extract hooks**: Reusable stateful logic → `use-<name>.ts` (e.g., `use-session-manager.ts`)
- **Extract sub-components**: Visual sections → named component files (e.g., `ThemePanel`, `KeybindsPanel`)
- **Extract pure logic**: Reducers, validators, parsers → standalone modules
- **Extract types**: Large type definitions → `types.ts` co-located with the domain

### SOLID

- **Single Responsibility**: Each file, function, and component does one thing. A component renders UI *or* manages state — not both. A hook manages one concern (sessions, detection, streaming), not the entire app.
- **Open/Closed**: Extend behavior through composition (tool registry, theme system, slash commands) rather than modifying existing code. New tools plug in via side-effect imports, not by editing a switch statement.
- **Liskov Substitution**: Subtypes must be substitutable. Use discriminated unions (`StreamEvent`, `AssistantPart`) so consumers handle all variants through exhaustive checks, not `instanceof`.
- **Interface Segregation**: Keep interfaces focused. A hook returns only what callers need — don't return the entire store. Props should be minimal: pass specific callbacks, not whole objects.
- **Dependency Inversion**: Depend on abstractions. Components receive data + callbacks via props, not direct store access. Hooks like `useSessionManager` accept an options object, not global singletons.

### DRY — Don't Repeat Yourself

- **Shared UI**: Reuse existing `data-slot` CSS selectors across components (e.g., theme cards in both settings and onboarding wizard share `settings-dialog.css`).
- **Shared logic**: Extract repeated patterns into hooks or utility functions. If the same 5+ lines appear in 2+ places, extract.
- **Shared types**: Define types once in the domain's `types.ts`, import everywhere. Never redeclare the same shape.
- **But avoid premature abstraction**: Three similar lines are better than a premature helper. DRY applies when the duplication is *exact and intentional*, not when code merely looks similar.

### KISS — Keep It Simple

- **No over-engineering**: Don't add abstractions, wrappers, or indirection layers for hypothetical future needs. Solve today's problem.
- **Flat over nested**: Prefer early returns over deeply nested conditionals. Prefer composition over inheritance.
- **Explicit over clever**: No meta-programming, Proxy objects, or dynamically generated code. Readable > concise.
- **Minimal state**: Derive values from existing state instead of storing redundant copies. Use `useMemo` for derived computations, not extra state variables.
- **Delete dead code**: Unused exports, commented-out blocks, and backwards-compatibility shims should be removed, not left "just in case."

### Composition over configuration

- **Hooks compose**: Small, focused hooks combine in components (`useSessionManager` + `useDetection` + `useTheme` in `App.tsx`).
- **Components compose**: `<ChatLayout>` renders `<SessionTurn>` renders `<BasicTool>` renders tool-specific parts.
- **CSS composes**: `data-slot` selectors are reusable building blocks. Atomic Tailwind classes compose for one-off layouts.

---

## Code Standards

### Rust

- **Error handling**: Use `anyhow::Result` for application errors, `thiserror` for library errors. Avoid manual `StatusCode` + JSON error responses — prefer Axum's `IntoResponse` trait implementations.
- **Async I/O**: Use `tokio::fs` for file operations in async contexts. `std::fs` is acceptable only in synchronous helper functions called outside the async runtime.
- **Locking**: Use `tokio::sync::Mutex` for state shared across `.await` points. Use `std::sync::Mutex` only for synchronous-only access. Prefer `RwLock` when reads outnumber writes.
- **Process management**: Always handle `Child` process cleanup. Use `tokio::select!` for timeout + process wait patterns. Send SIGTERM before SIGKILL.
- **Serde**: Derive `Serialize`/`Deserialize` with `#[serde(rename_all = "snake_case")]`. Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields.
- **File size**: Keep modules under 300 lines (see Design Principles). Split into submodules with `mod.rs` re-exports when approaching the limit.
- **Clippy**: Run `cargo clippy -- -W clippy::all` before committing. Fix all warnings.
- **Tests**: Use `#[tokio::test]` for async tests. Avoid `set_current_dir` in tests (causes race conditions).

### TypeScript / React

**Type safety (zero tolerance)**:
- **NEVER use `any`** — use `unknown`, proper types, or `Record<string, unknown>`.
- **NEVER use `as` type assertions** — use type guards (`is`), discriminated unions, or proper narrowing. The only acceptable `as` is `as const` and React's `as React.CSSProperties` for CSS custom properties.
- **Discriminated unions**: Use `type` field for union discrimination (see `StreamEvent`, `AssistantPart`).
- **JSON parsing**: Always parse to `unknown` first, then narrow with type guards. Never `JSON.parse(x) as Foo`.
- **Error handling**: Prefer discriminated union results over try/catch for operations with multiple failure modes (network, abort, API errors). Use `safeFetch` (`@/shared/api/safe-fetch`) for fetch operations. Use exhaustive `switch` + `default: never` instead of `instanceof` chains. Reserve try/catch for truly unexpected exceptions only.

**Style (Deno style guide)**:
- **Naming**: `camelCase` for functions/variables, `PascalCase` for types/classes, `UPPER_SNAKE_CASE` for top-level constants. Acronyms follow standard casing (`HttpObject`, not `HTTPObject`).
- **Exported functions**: Max 2 required args, put the rest into an options object. Export all interfaces used as params or return types.
- **Top-level functions**: Use `function` keyword, not arrow syntax. Arrow functions are fine for callbacks and local closures.
- **Minimize dependencies**: Do not make circular imports. No meta-programming or Proxy usage — keep code explicit.
- **Private fields**: Prefer `#field` syntax over TypeScript `private` keyword.
- **Error messages**: Start with uppercase, no ending period. Use active voice (e.g., `"Cannot connect to session"`, not `"connection failed."`). Quote values (e.g., `"Cannot find session 'abc'"`).
- **TODO comments**: Always reference an issue number or GitHub username — `// TODO(mbourmaud): description` or `// TODO(#123): description`.
- **Tests**: Add tests for new features. Test names should be explicit and descriptive.

**React patterns**:
- **Hooks**: Follow rules of hooks strictly. Only wrap in `useCallback`/`useMemo` when there's a measurable perf benefit or a dependency requires stable references.
- **Components**: Use `data-component` and `data-slot` CSS selectors (not className-based styling) for component structure. Keep components focused — extract when approaching 300 lines (see Design Principles).
- **Events**: Always clean up event listeners, timers, observers in `useEffect` return functions.
- **Keys**: Use stable, unique IDs for list keys — never array indices.

**Linting**: Biome enforces these rules via `web/biome.json`. Run `npm run lint` in `web/` before committing.

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

### Safe Fetch (discriminated union error handling)
Use `safeFetch()` from `@/shared/api/safe-fetch` instead of raw `fetch()` + try/catch when the caller needs to distinguish failure modes. Returns `FetchResult`:
- `{ ok: true, response }` — success, read the body as needed
- `{ ok: false, type: "aborted" }` — user cancelled (AbortController)
- `{ ok: false, type: "network", message }` — DNS, offline, CORS, etc.
- `{ ok: false, type: "api", status, message }` — server returned non-2xx

Handle with exhaustive `switch` + `default: never`. This replaces nested try/catch, `instanceof DOMException` checks, and duplicate error dispatch. The throwing `apiClient` is still used for simple CRUD where abort handling doesn't matter.

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
