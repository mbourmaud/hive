# Hive Plan - Collaborative Planning for Drones

Create a plan JSON file collaboratively with the user, using Claude's plan mode for iterative exploration and refinement.

## What this does

Creates a plan JSON file in `.hive/plans/` that a Hive drone can execute autonomously. Unlike the old PRD workflow, plans are **freeform markdown** — no rigid stories or DoD cards. The plan is crafted collaboratively through conversation.

## Workflow

### Step 1: Check Hive Initialization

First, check if `.hive/` exists in the project. If not, tell the user to run `hive init`.

### Step 2: Enter Plan Mode

Enter Claude plan mode to explore the codebase and design the plan collaboratively:

1. Ask the user: **"What do you want to build or change?"**
2. Explore the codebase to understand the project structure, patterns, and affected areas
3. Discuss the approach with the user — ask clarifying questions, propose alternatives
4. Iterate on the plan until the user is satisfied

### Step 3: Draft the Plan

As you explore and discuss, build up a freeform markdown plan. The plan should include:

- **Goal**: What we're trying to achieve (1-2 sentences)
- **Requirements**: Key things that must be true when done
- **Approach**: How to implement it (high-level steps, architectural decisions)
- **Files affected**: Which parts of the codebase will change
- **Testing strategy**: How to verify the work
- **PR/MR story** (if applicable): Creating the merge request and ensuring CI passes

The plan is **freeform markdown** — use whatever structure makes sense for the task. Simple tasks need simple plans. Complex tasks can have more detail.

### Step 4: Plan Metadata

Determine the plan metadata:
- **ID**: Short kebab-case identifier derived from the goal (e.g., `add-jwt-auth`, `fix-payment-bug`)
- **Title**: Human-readable title
- **Description**: Brief summary (1 sentence)
- **Target Branch** (optional): Git branch name (default: `hive/{id}`)
- **Base Branch** (optional): Branch to create worktree from (default: auto-detect main/master)

### Step 5: Write the Plan File

Write the JSON file to `.hive/plans/plan-<id>.json`:

```json
{
  "id": "add-jwt-auth",
  "title": "Add JWT Authentication",
  "description": "Add JWT-based authentication to the API endpoints",
  "version": "1.0.0",
  "created_at": "2025-01-15T12:00:00Z",
  "target_branch": "hive/add-jwt-auth",
  "plan": "## Goal\nAdd JWT authentication to all API routes.\n\n## Requirements\n- All /api/* routes require a valid JWT token\n- Tokens are verified against the secret in .env\n- Unauthorized requests return 401\n\n## Approach\n1. Create auth middleware in src/middleware/auth.ts\n2. Add middleware to all API route handlers\n3. Write tests for auth scenarios (401 without token, 200 with valid token)\n4. Update .env.example with JWT_SECRET\n\n## Testing\n- Unit tests for middleware\n- Integration tests for each protected route\n- Run full test suite to check for regressions"
}
```

### Step 6: Exit Plan Mode and Next Steps

Tell the user:

```
Plan ready! Saved to .hive/plans/plan-<id>.json

To launch a drone:
  hive start <id>

To monitor progress:
  hive monitor
```

## Plan JSON Schema

```typescript
interface Plan {
  id: string;                    // kebab-case identifier
  title: string;                 // Human-readable title
  description?: string;          // Brief summary
  version?: string;              // e.g., "1.0.0"
  created_at?: string;           // ISO timestamp
  target_platforms?: string[];   // Optional platform targets
  target_branch?: string;        // Git branch (default: hive/{id})
  base_branch?: string;          // Base branch for worktree
  plan: string;                  // Freeform markdown plan
}
```

## Guidelines

1. **Keep it conversational** — this is a dialogue, not a wizard
2. **Explore before planning** — understand the codebase first
3. **The plan field is freeform markdown** — use whatever structure fits the task
4. **Simple tasks = simple plans** — don't over-engineer a 2-line bug fix
5. **Complex tasks = detailed plans** — architecture decisions, testing strategy, rollback plan
6. **Always include a testing section** — how will the drone verify its work?
7. **Use kebab-case for the plan ID** — it becomes part of the filename and branch name
