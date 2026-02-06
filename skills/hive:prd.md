# Hive PRD - Generate a PRD for Drones

Generate a PRD (Product Requirements Document) JSON file optimized for Hive drones using the enhanced v2.x schema.

## What this does

Creates a PRD JSON file in `.hive/prds/` with well-defined stories that a drone can execute autonomously. Each story has a clear **Definition of Done** validated with the user.

## Workflow

### Step 1: Check Hive Initialization

First, check if `.hive/` exists in the project. If not, run `hive init`.

### Step 2: Understand the Feature

Ask the user: **"What feature or task do you want to break down into a PRD?"**

Let them describe in natural language what they want to accomplish.

### Step 3: Explore the Codebase

Before writing stories, explore the codebase to understand:
- Project structure
- Existing patterns
- Files that will be affected
- Dependencies
- Build system and package managers
- Environment configuration needs
- Pre-commit hooks and linters

**If Agent Teams is available** (CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1 in settings), use teammates to parallelize the exploration for large/complex projects. Spawn multiple teammates to investigate different aspects simultaneously:

- **Teammate 1**: Project structure & architecture (scan directories, read config files, understand module layout)
- **Teammate 2**: Dependencies & build system (package.json, Cargo.toml, Makefile, CI config, env files, git hooks)
- **Teammate 3**: Affected code areas (search for files/patterns related to the feature the user described)

Use the Task tool with `subagent_type: "Explore"` to spawn parallel exploration agents. Each teammate reports back findings that inform the story breakdown.

**If Agent Teams is not available**, or for simpler projects/small features, use sequential exploration with Glob, Grep, and Read tools as before.

### Step 4: PRD Metadata

Ask the user for:
- **PRD ID** - Short kebab-case identifier (e.g., `security-api-protection`)
- **Title** - Human-readable title
- **Description** - Brief summary of the overall goal
- **Target Branch** (optional) - Git branch name (default: `hive/{id}`)

### Step 5: Break Down Feature into Stories

Break down the feature into logical stories. These will be US-1, US-2, US-3, etc.

For **EACH story**, follow the interactive DoD process described in Step 6 below.

### Step 6: Interactive DoD for Each Story

For **EACH story**, present a **compact card** using the format below. The goal is to make each story scannable in under 10 seconds.

#### 6a. Present the story card

Use this exact format - a single compact block with clear visual sections:

```
┌─────────────────────────────────────────────────┐
│ SEC-001 · Protect /api/accounts/*               │
├─────────────────────────────────────────────────┤
│ Add requireAuth() middleware to account routes   │
│                                                  │
│ Files: src/app/api/accounts/route.ts             │
│        src/app/api/accounts/__tests__/auth.test  │
│ Deps:  none                                      │
│                                                  │
│ Definition of Done:                              │
│  1. requireAuth() added to GET and POST routes   │
│  2. Test: GET /api/accounts → 401 without auth   │
│  3. Test: GET /api/accounts → 200 with auth      │
│                                                  │
│ Verify:                                          │
│  $ grep -r "requireAuth" src/app/api/accounts/   │
│  $ pnpm test --filter=accounts                   │
└─────────────────────────────────────────────────┘
❓ DoD OK? (anything to add/change/remove?)
```

**Card rules:**
- **Line 1**: `{ID} · {Title}` - always on one line
- **Description**: 1-2 sentences max. If longer, you're putting too much detail in the description - move it to actions
- **Files**: List affected files, one per line. Use short paths (trim common prefix if obvious)
- **Deps**: Show `depends_on` story IDs, or `none`
- **DoD**: Numbered list. Each item must be **verifiable** - something you can check with a command or grep. Keep items short (one line each). Aim for 2-5 items
- **Verify**: Actual shell commands the drone will run. Prefix with `$`
- **Question**: Short, one-line prompt

#### 6b. Iterate until user validates
The user might say:
- "Need to also log unauthenticated attempts"
- "Add an e2e test"
- "OK looks good"

Update the card based on feedback. Re-display the full updated card so the user can see the final state, then move to next story.

### Step 7: Add MR/CI Story (Standard)

**ALWAYS** add a final story for creating and managing the Merge Request / Pull Request. This is a standard story that should be included in every PRD.

Ask the user: **"Should I include a story for creating the MR and monitoring the CI pipeline?"**

Options:
- **Yes (Recommended)** - Include MR-001 story
- **No** - Skip (rare cases like documentation-only PRDs)

If yes, add this standard story at the END of the stories array:

```json
{
  "id": "MR-001",
  "title": "Create MR and ensure CI passes",
  "description": "Create the Merge Request on GitLab/GitHub, monitor the CI pipeline, fix any failures, and ensure the MR is ready for review",
  "definition_of_done": [
    "A MR/PR is created with a clear title and description",
    "The CI pipeline passes (lint, tests, build)",
    "If pipeline fails, errors are analyzed, fixed, and pipeline re-run",
    "The MR is ready for review (no conflicts, green pipeline)"
  ],
  "verification_commands": [
    "glab ci status || gh pr checks",
    "git status"
  ],
  "actions": [
    "Create MR with 'glab mr create' or PR with 'gh pr create'",
    "Monitor pipeline status",
    "If pipeline fails: analyze logs, fix issues, commit, push",
    "Rebase on target branch if needed to resolve conflicts"
  ]
}
```

**Note**: Adapt the commands based on the project's Git platform (GitLab → `glab`, GitHub → `gh`).

### Step 8: Intelligently Determine US-0 Environment Setup

**IMPORTANT**: After defining all feature stories (US-1, US-2, etc.), analyze whether a **US-0: Environment Setup** story is needed.

#### Auto-Discovery Process:

Based on:
1. Your codebase exploration in Step 3
2. The stories you just created (US-1, US-2, etc.)
3. The verification commands and tools they require

Intelligently determine if US-0 is needed by checking:

**Package Managers & Dependencies:**
- `package.json` → npm/pnpm/yarn install needed
- `requirements.txt`, `pyproject.toml` → pip/poetry install needed
- `Cargo.toml` → cargo build needed
- `go.mod` → go mod download needed
- `composer.json` → composer install needed

**Build Systems:**
- `tsconfig.json`, `webpack.config.js` → build step needed
- `Makefile` → make commands needed
- `.github/workflows/` → CI commands to replicate
- Do the stories' verification commands require a build?

**Environment Files:**
- `.env.example`, `.env.template` → .env copy/setup needed
- `config/*.example` → config files to copy
- Do the stories access environment variables?

**Git Hooks & Linters:**
- `.husky/`, `.git/hooks/` → pre-commit hooks to install
- `.eslintrc`, `.prettierrc` → linters to verify
- `pre-commit-config.yaml` → pre-commit framework
- Will commits be blocked without setup?

**LSP & Tooling:**
- Do the stories involve TypeScript/Python/etc. code? → LSP needs dependencies
- Will imports work without node_modules/venv?
- Do tests need to run? → test framework setup needed

#### Decision Logic:

**Create US-0 if ANY of these are true:**
- Package manager files exist AND stories involve writing/modifying code
- Build steps are required AND stories' verification commands will need to build
- Environment files are needed for features to work
- Pre-commit hooks exist (they'll block commits if not set up)
- Stories require LSP functionality (imports, type checking, auto-complete)
- Tests need to run as verification

**Skip US-0 if:**
- PRD is documentation-only
- Stories only modify existing files without new dependencies
- It's a simple config change with no tooling

#### If US-0 is Needed:

Generate the US-0 story using the **same card format** as Step 6:

```
┌─────────────────────────────────────────────────┐
│ US-0 · Environment Setup                        │
├─────────────────────────────────────────────────┤
│ Prepare worktree for autonomous development     │
│                                                  │
│ Files: (none — environment only)                 │
│ Deps:  none (runs first)                         │
│                                                  │
│ Definition of Done:                              │
│  1. Dependencies installed (pnpm install)        │
│  2. Build passes (pnpm build)                    │
│  3. .env configured (from .env.example)          │
│  4. Husky hooks installed (pnpm prepare)         │
│  5. Tests can run (pnpm test finds tests)        │
│                                                  │
│ Verify:                                          │
│  $ test -d node_modules && echo "OK"             │
│  $ test -f .env && echo "OK"                     │
│  $ pnpm build                                    │
│  $ pnpm test                                     │
│  $ git diff --exit-code                          │
└─────────────────────────────────────────────────┘
❓ DoD OK? (anything to add/change/remove?)
```

Only include DoD items that are actually relevant (discovered in Step 3). Don't add generic items - each item should be justified by what you found in the codebase.

**IMPORTANT**: US-0 goes FIRST in the stories array, even though you define it last. The drone will execute stories in array order.

### Step 9: Review Complete PRD

Once all stories (including US-0 if applicable) have validated DoDs, present a **full summary table** followed by a **dependency graph**. This is the user's last chance to review before generation - make it count.

#### 9a. Summary table

```
╔══════════════════════════════════════════════════════════════════════╗
║  PRD: security-api-protection                                       ║
║  "Secure API Routes" — 5 stories                                    ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║  #   │ Story                        │ Files │ DoD │ Deps             ║
║  ────┼──────────────────────────────┼───────┼─────┼────────────────  ║
║  US-0│ Environment Setup            │   0   │  5  │ —                ║
║  S-01│ Protect /api/accounts/*      │   2   │  3  │ US-0             ║
║  S-02│ Protect /api/users/*         │   2   │  3  │ US-0             ║
║  S-03│ Add auth logging             │   3   │  4  │ S-01, S-02       ║
║  MR-1│ Create MR & CI              │   0   │  4  │ S-03             ║
║                                                                      ║
║  Parallelism: S-01 ‖ S-02 (can run together)                       ║
║  Total files affected: 7 unique files                                ║
║  Estimated drone stories: 5                                          ║
║                                                                      ║
╚══════════════════════════════════════════════════════════════════════╝
```

**Table rules:**
- **#**: Story ID (shortened if long, e.g., SEC-001 → S-01 for readability)
- **Story**: Title only, kept short
- **Files**: Count of files affected (not the list - that's in the cards)
- **DoD**: Count of DoD items
- **Deps**: Story IDs this depends on, or `—` for none
- **Parallelism line**: Clearly show which stories can run in parallel (indicated by `‖`)
- **Total files**: De-duplicated count across all stories

#### 9b. Dependency flow (if >2 stories)

Show the execution order visually:

```
Execution flow:
  US-0 → ┬─ S-01 ─┬─ S-03 → MR-1
         └─ S-02 ─┘
```

This makes it immediately clear: what runs first, what's parallel, what's sequential.

#### 9c. Quick DoD recap (optional, for complex PRDs with 5+ stories)

If the PRD has many stories, add a collapsed view the user can mentally scan:

```
DoD recap:
  US-0: deps installed, build works, env ready, hooks installed, tests run
  S-01: middleware added, 401 test, 200 test
  S-02: middleware added, 401 test, 200 test
  S-03: logger created, logs on 401, log format valid, log rotation configured
  MR-1: PR created, CI green, no conflicts, ready for review
```

Each line is a **one-line summary** of the DoD items (not the full text - just the essence).

#### 9d. Ask for final validation

Ask: **"Does this PRD look good? Should I generate the file?"**

If the user wants to change something, go back to the relevant story card (Step 6) and iterate. Do NOT regenerate the entire PRD review for a single story change - just show the updated card, then re-show the summary table with the change highlighted.

### Step 10: Generate PRD File

Write the JSON file to `.hive/prds/prd-<id>.json` using the **current v2.x enhanced schema**:

```json
{
  "id": "security-api-protection",
  "title": "Secure API Routes",
  "description": "Add authentication to all API routes",
  "version": "1.0.0",
  "created_at": "2024-01-21T12:00:00Z",
  "target_branch": "feature/api-security",
  "stories": [
    {
      "id": "US-0",
      "title": "Environment Setup",
      "description": "Prepare worktree environment for autonomous development",
      "definition_of_done": [
        "All dependencies installed successfully",
        "Build commands execute without errors",
        "Environment files configured",
        "Pre-commit hooks installed",
        "LSP servers functional",
        "All verification commands for subsequent stories will work"
      ],
      "verification_commands": [
        "test -d node_modules && echo 'Dependencies OK'",
        "test -f .env && echo 'Environment OK'",
        "pnpm build",
        "pnpm test",
        "git diff --exit-code"
      ],
      "actions": [
        "Run pnpm install",
        "Copy .env.example to .env",
        "Run pnpm build to verify tooling",
        "Install husky hooks with pnpm prepare",
        "Test that pnpm test works"
      ],
      "tools": [
        "pnpm",
        "husky"
      ]
    },
    {
      "id": "SEC-001",
      "title": "Protect /api/accounts/*",
      "description": "Add requireAuth() middleware to account routes",
      "definition_of_done": [
        "requireAuth() middleware is added to GET and POST routes",
        "A test verifies that GET /api/accounts returns 401 without auth",
        "A test verifies that GET /api/accounts returns 200 with auth"
      ],
      "verification_commands": [
        "grep -r 'requireAuth' src/app/api/accounts/",
        "pnpm test --filter=accounts"
      ],
      "files": [
        "src/app/api/accounts/route.ts",
        "src/app/api/accounts/__tests__/auth.test.ts"
      ],
      "actions": [
        "Import requireAuth middleware",
        "Add middleware to route handlers",
        "Write unit tests for auth scenarios",
        "Run tests to verify"
      ],
      "tools": [
        "jest",
        "pnpm"
      ]
    },
    {
      "id": "MR-001",
      "title": "Create MR and ensure CI passes",
      "description": "Create the Merge Request, monitor CI pipeline, fix failures, ensure MR is ready for review",
      "definition_of_done": [
        "A MR/PR is created with a clear title and description",
        "The CI pipeline passes (lint, tests, build)",
        "If pipeline fails, errors are analyzed, fixed, and pipeline re-run",
        "The MR is ready for review (no conflicts, green pipeline)"
      ],
      "verification_commands": [
        "glab ci status || gh pr checks",
        "git status"
      ],
      "actions": [
        "Create MR with 'glab mr create' or PR with 'gh pr create'",
        "Monitor pipeline status",
        "If pipeline fails: analyze logs, fix issues, commit, push",
        "Rebase on target branch if needed to resolve conflicts"
      ]
    }
  ]
}
```

**IMPORTANT**: For comprehensive PRDs, consider using the **enhanced schema** with additional fields:
- `actions` - Step-by-step actions to take
- `files` - Specific files to modify
- `tools` - Tools/commands to use
- `context` - Dependencies, prerequisites, architectural notes
- `testing` - Unit/integration/e2e test requirements
- `error_handling` - Expected errors and recovery strategies
- `agent_controls` - Max iterations, approval requirements
- `communication` - Commit/PR templates, docs to update

See the full schema reference in `docs/PRD_GUIDE.md` or the example at `examples/prd-enhanced-example.json`.

### Step 11: Next Steps

Tell the user:
```
PRD created: .hive/prds/prd-security-api-protection.json

To launch a drone on this PRD:
  hive start security-api-protection

To monitor the drone:
  hive monitor
```

## PRD JSON Schema (v2.x)

### Minimal Schema (Required Fields Only)

```typescript
interface PRD {
  id: string;              // kebab-case identifier
  title: string;           // Human-readable title
  description?: string;    // Overall goal
  stories: Story[];
}

interface Story {
  id: string;                    // Unique ID (e.g., "SEC-001")
  title: string;                 // Short title
  description?: string;          // What to implement
  definition_of_done: string[];  // Clear, validated DoD statements
  verification_commands: string[]; // Commands drone MUST run to prove completion
  depends_on?: string[];         // Story IDs that must complete before this one starts
  parallel?: boolean;            // Whether this story can run in parallel with others
}
```

### Enhanced Schema (All Fields)

```typescript
interface PRD {
  id: string;
  title: string;
  description?: string;
  version?: string;                    // e.g., "1.0.0"
  created_at?: string;                 // ISO timestamp
  target_platforms?: string[];         // ["web", "mobile", "api"]
  target_branch?: string;              // Git branch (default: hive/{id})
  stories: Story[];
}

interface Story {
  // Core fields
  id: string;
  title: string;
  description?: string;
  acceptance_criteria?: string[];      // User-facing criteria
  definition_of_done: string[];        // Technical completion criteria
  verification_commands: string[];     // Shell commands to verify
  notes?: string;

  // Enhanced guidance fields
  actions?: string[];                  // Step-by-step actions
  files?: string[];                    // Files to modify/create
  tools?: string[];                    // Tools/commands to use

  // Context fields
  context?: {
    dependencies?: string[];           // External dependencies
    prerequisites?: string[];          // Must be done first
    architectural_notes?: string[];    // Patterns to follow
    related_docs?: string[];           // Doc references
  };

  // Testing fields
  testing?: {
    unit_tests?: string[];             // Required unit tests
    integration_tests?: string[];      // Required integration tests
    e2e_tests?: string[];              // Required e2e tests
    coverage_threshold?: number;       // Min coverage % (0-100)
  };

  // Error handling fields
  error_handling?: {
    expected_errors: string[];         // Expected error scenarios
    rollback_procedure?: string;       // How to rollback
    recovery_strategy?: string;        // How to recover
  };

  // Agent control fields
  agent_controls?: {
    max_iterations?: number;           // Max iterations before blocking
    require_approval_for?: string[];   // Actions needing approval
    block_on?: string[];               // Conditions to block on
  };

  // Communication fields
  communication?: {
    commit_template?: string;          // Git commit message template
    pr_template?: string;              // Pull request template
    docs_to_update?: string[];         // Documentation to update
    changelog_entry?: string;          // CHANGELOG.md entry
  };

  // Dependency & parallelism fields
  depends_on?: string[];               // Story IDs that must complete first
  parallel?: boolean;                  // Can run in parallel with other stories
}
```

### Dependency Graph Example

Stories can declare dependencies on other stories for proper ordering:

```json
{
  "stories": [
    {
      "id": "DB-001",
      "title": "Create database schema",
      "parallel": false
    },
    {
      "id": "API-001",
      "title": "Implement REST endpoints",
      "depends_on": ["DB-001"],
      "parallel": true
    },
    {
      "id": "API-002",
      "title": "Add authentication middleware",
      "depends_on": ["DB-001"],
      "parallel": true
    },
    {
      "id": "INT-001",
      "title": "Integration tests",
      "depends_on": ["API-001", "API-002"]
    }
  ]
}
```

In this example:
- `DB-001` runs first (no dependencies)
- `API-001` and `API-002` can run in parallel after `DB-001` completes
- `INT-001` waits for both API stories to complete

## Definition of Done Guidelines

A good DoD must be:

1. **Verifiable** - Can be proven by a command or check
2. **Specific** - Not "it works well", but "returns 200 with body {x: y}"
3. **Complete** - Includes tests, commits, and all required actions
4. **Validated** - User has explicitly confirmed

### Examples of GOOD DoD items:
- "The file `src/auth.ts` contains the function `validateToken()`"
- "Tests pass: `pnpm nx test plato --testPathPattern=auth`"
- "A commit is created with message `feat(auth): add token validation`"
- "Route GET /api/users returns 401 without Authorization header"

### Examples of BAD DoD items:
- "Authentication works" (too vague)
- "Code is clean" (subjective)
- "Everything is tested" (not specific)

## Verification Commands

Each story SHOULD have verification commands that the drone will execute to PROVE the story is complete. These are not optional - they are mandatory checks.

**Important**: `verification_commands` is an array of strings (shell commands), NOT objects. Each command should be executable as-is.

Types of verification:
- **File existence**: `test -f src/auth.ts && echo "OK"`
- **Code presence**: `grep -q "functionName" src/file.ts && echo "OK"`
- **Tests pass**: `pnpm test --filter=module`
- **Build succeeds**: `pnpm build`
- **Git status**: `git diff --name-only` to verify files changed
- **API check**: `curl -s localhost:3000/api/health | jq .status`

## File Location

PRDs are stored in `.hive/prds/` which is:
- Gitignored (not committed by default)
- Shared via symlink with drone worktrees
- Accessible by both main project and drones

## Interactive Clarification Questions

When defining DoD, ask clarifying questions like:

- "Does this story need a unit test, integration test, or both?"
- "Should there be a separate commit or can it be grouped?"
- "Are there side effects to verify?"
- "What command can verify this is done?"
- "Should the drone also update documentation?"

## Schema Recommendation

For **simple, straightforward** tasks:
- Use minimal schema (id, title, definition_of_done, verification_commands)
- Keep it lightweight and fast

For **complex, critical** features:
- Use enhanced schema with context, testing, error_handling, agent_controls
- Provides more guidance and safety
- Better for production-critical work

Refer to:
- `docs/PRD_GUIDE.md` - Complete schema documentation
- `examples/prd-enhanced-example.json` - Full example with all fields

## Important Rules

1. **NEVER generate a story without user validation of its DoD**
2. **ALWAYS include at least one verification command per story**
3. **ASK questions when the DoD is ambiguous**
4. **ITERATE until the user says "OK" or "looks good"**
5. **Use string arrays for verification_commands**, not objects
6. **Follow the current v2.x schema** as defined in `src/types.rs`
