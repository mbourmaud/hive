# Hive Plan - Collaborative Planning for Drones

Create a markdown plan file collaboratively with the user, then offer to launch a drone immediately. Uses behavioral constraints (no plan mode API) for iterative exploration and an interactive approval flow before launch.

## What this does

Creates a markdown plan file in `.hive/plans/` that a Hive drone can execute autonomously. Plans use **structured markdown** with metadata per task for automated setup, task pre-seeding, and PR/MR handling. The plan is crafted collaboratively through conversation, then **you MUST offer to launch a drone via `/hive:start`** — never tell the user to implement it themselves.

## Workflow

### Step 1: Check Hive Initialization

First, check if `.hive/` exists in the project. If not, tell the user to run `hive init`.

### Step 2: Explore and Plan (Behavioral Constraints)

You are in **PLANNING PHASE**. During this phase:

**ALLOWED:**
- Read, Grep, Glob, read-only Bash (git log, cargo check, ls, cat)
- Discuss with user, AskUserQuestion

**NOT ALLOWED:**
- Write, Edit any project files
- Run state-modifying commands (cargo build, npm install, git commit, etc.)
- The ONLY file you write is `.hive/plans/<id>.md`

DO NOT use `EnterPlanMode` or `ExitPlanMode`. These are platform APIs that cause bugs in the planner flow.

- If the user invoked `/hive:plan <prompt>` with a prompt, use that as the starting point and begin exploring the codebase immediately.
- If the user invoked bare `/hive:plan`, ask: **"What do you want to build or change?"**

Then:
1. Explore the codebase to understand the project structure, patterns, and affected areas
2. **Detect the project type** — look for `package.json` (pnpm/npm/yarn), `Cargo.toml`, `go.mod`, `pyproject.toml`, etc. This determines setup and CI commands.
3. **Detect the git hosting platform** — run `git remote get-url origin` and identify the platform from the URL:
   - `github.com` → GitHub — use `gh pr create`
   - `gitlab` → GitLab — use `glab mr create`
   - `bitbucket` → Bitbucket — push only, or use `bb pr create` if available
   - Other/self-hosted → push only, skip PR creation
   This is **CRITICAL** — using the wrong CLI tool will crash the drone (e.g. `gh` on a GitLab repo). Use the detected platform throughout the plan for PR/MR creation and CI commands.
4. Discuss the approach with the user — ask clarifying questions, propose alternatives
5. Iterate on the plan until the user is satisfied

### Step 3: Draft the Plan

As you explore and discuss, build up a markdown plan. **Every plan MUST include these sections:**

#### Required sections

1. **Goal**: What we're trying to achieve (1-2 sentences)
2. **Tasks**: Ordered list of work items using `### N. Title` subsections. **The first and last tasks are MANDATORY:**
   - **Task 1 — Environment Setup** (ALWAYS FIRST): Install dependencies, verify project builds/compiles, run codegen if needed. Mark with `- type: setup`.
   - **Task N — PR/MR & CI** (ALWAYS LAST): Lint/format, run tests, commit, push, create PR/MR, verify CI passes. Mark with `- type: pr`.
   - In between: the actual implementation tasks (default type: `work`).
3. **Definition of Done**: Explicit, verifiable checklist that Claude can use to confirm the work is complete. This is **CRITICAL** — without it, Claude will not know when to stop or what to validate.

#### Structured task metadata

Each task under `## Tasks` uses `### N. Title` headings. Immediately after the heading, you can add **metadata bullets** (`- key: value`) to control how Hive handles the task:

| Key | Values | Default | Description |
|-----|--------|---------|-------------|
| `type` | `setup`, `pr`, `work` | `work` | `setup` = Hive runs env setup before launch. `pr` = Hive handles PR/MR after work completes. `work` = dispatched to teammates. |
| `model` | `opus`, `sonnet`, `haiku` | CLI `--model` or `sonnet` | Model for the teammate executing this task |
| `parallel` | `true`, `false` | `false` | Whether this task can run concurrently with other parallel tasks |
| `files` | comma-separated paths | (none) | Files this task will modify (prevents multi-agent file conflicts) |
| `depends_on` | comma-separated task numbers | (none) | Task numbers that must complete before this one can start |

**All metadata keys are optional.** After the metadata bullets, add the task description as regular markdown text.

#### Optional sections (add as needed)

- **Requirements**: Key things that must be true when done
- **Approach**: Architectural decisions, implementation strategy
- **Files affected**: Which parts of the codebase will change
- **Risks / Edge cases**: Things to watch out for

Use whatever structure makes sense for the task. Simple tasks need simple plans. Complex tasks can have more detail. But the three required sections (Goal, Tasks with first/last, Definition of Done) are **non-negotiable**.

### Step 4: Determine Plan ID and Branches

- **ID**: Short kebab-case identifier derived from the goal (e.g., `add-jwt-auth`, `fix-payment-bug`). This becomes the filename and drone name.
- **Target Branch** (optional): If needed, add a YAML frontmatter block at the top of the markdown:

```markdown
---
target_branch: hive/add-jwt-auth
base_branch: main
---
```

If no frontmatter is needed, just write pure markdown.

### Step 5: Write the Plan File

Write the markdown file to `.hive/plans/<id>.md`. Here is a **complete example** using structured tasks:

```markdown
# Add JWT Authentication

## Goal
Add JWT-based authentication to all API routes so that unauthorized requests are rejected with 401.

## Tasks

### 1. Environment Setup
- type: setup

### 2. Create auth middleware
- model: sonnet
- parallel: true
- files: src/middleware/auth.ts

Create `src/middleware/auth.ts` with JWT verification logic.
Use the `jsonwebtoken` package (already in dependencies).
Verify tokens against `JWT_SECRET` from environment.
Return 401 with `{ error: "Unauthorized" }` for invalid/missing tokens.

### 3. Apply middleware to routes
- model: sonnet
- parallel: true
- files: src/routes/users.ts, src/routes/orders.ts
- depends_on: 2

Add auth middleware to all `/api/*` route handlers.
Exclude `/api/auth/login` and `/api/auth/register` from protection.

### 4. Write tests
- model: haiku
- depends_on: 2, 3

Unit tests for the auth middleware (valid token, expired token, missing token, malformed token).
Integration tests for protected routes (with and without token).
Add test helper for generating test JWT tokens.

### 5. PR & CI
- type: pr
- depends_on: 2, 3, 4

## Definition of Done
- [ ] `pnpm build` succeeds with no errors
- [ ] All `/api/*` routes (except auth) return 401 without a valid JWT
- [ ] Valid JWT tokens grant access to protected routes
- [ ] Unit tests cover: valid token, expired token, missing token, malformed token
- [ ] Integration tests verify end-to-end auth flow
- [ ] `pnpm lint` passes with no warnings
- [ ] `pnpm test` passes with all tests green
- [ ] PR is created and CI pipeline is green
```

**What Hive does with structured tasks:**
- `type: setup` → Hive runs `pnpm install` / `cargo check` / etc. **before** launching the team lead
- `type: pr` → Hive injects the correct PR/MR command (auto-detected from git remote)
- `type: work` (default) → Pre-seeded as tasks in the team lead's task list
- `model` → Teammates use the specified model
- `parallel` → Tasks run concurrently when possible
- `depends_on` → Tasks are blocked until dependencies complete
- `files` → Prevents multi-agent file conflicts

### Step 6: Offer to Launch

After writing the plan file, present the user with an interactive launch flow using `AskUserQuestion`:

- **Question:** "Plan saved to `.hive/plans/<id>.md`! Ready to launch a drone?"
- **Options:**
  - "Launch drone now" (Recommended) — immediately invoke the `/hive:start` skill with the plan ID to launch the drone
  - "Save only" — confirm the plan is saved, done
- **Custom feedback (Other):** If the user types feedback, iterate on the plan based on their feedback, update the plan file, then re-present the same `AskUserQuestion` options

There is NO `ExitPlanMode`. Your turn ends after the user's choice is executed.

**CRITICAL — DO NOT IMPLEMENT THE PLAN YOURSELF:**
- When the user selects "Launch drone now", invoke `/hive:start` to launch a **drone** (a separate Claude Code instance in a worktree). You are NOT the drone. Do NOT write code, create files, or make changes yourself.
- When the user selects "Save only", just confirm the file is saved. Do NOT start implementing.
- **You are the planner, not the implementer.** Your job ends after saving the plan markdown and optionally launching a drone via `/hive:start`.

This creates a tight loop: plan → review → feedback → plan → review → launch drone.

## Guidelines

1. **Keep it conversational** — this is a dialogue, not a wizard
2. **Explore before planning** — understand the codebase first
3. **Detect the project type early** — this determines setup/CI commands in tasks 1 and N
4. **Always use structured tasks** — use `### N. Title` with metadata bullets. This is required for Hive to pre-seed tasks and run setup.
5. **Simple tasks = simple plans** — but ALWAYS include setup, implementation, PR/CI, and Definition of Done
6. **Complex tasks = detailed plans** — architecture decisions, testing strategy, rollback plan
7. **Definition of Done is mandatory** — use checkboxes (`- [ ]`) so the drone can verify each item
8. **Use kebab-case for the plan ID** — it becomes the filename and branch name
9. **The filename IS the drone name** — `.hive/plans/fix-auth.md` → `hive start fix-auth`
10. **Be specific in setup & CI tasks** — use the actual commands for the project (`pnpm`, `cargo`, `go`, etc.), not generic placeholders
11. **Use `depends_on` for ordering** — this prevents coordination chaos where tasks start before their dependencies finish
12. **Use `files` for ownership** — this prevents multiple agents from editing the same file simultaneously
