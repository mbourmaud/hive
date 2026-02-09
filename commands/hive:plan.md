# Hive Plan - Collaborative Planning for Drones

Create a plan JSON file collaboratively with the user, then offer to launch a drone immediately. Uses Claude's plan mode for iterative exploration and an interactive approval flow before launch.

## What this does

Creates a plan JSON file in `.hive/plans/` that a Hive drone can execute autonomously. Plans are **freeform markdown** — no rigid stories or DoD cards. The plan is crafted collaboratively through conversation, then **you MUST offer to launch a drone via `/hive:start`** — never tell the user to implement it themselves.

## Workflow

### Step 1: Check Hive Initialization

First, check if `.hive/` exists in the project. If not, tell the user to run `hive init`.

### Step 2: Enter Plan Mode

Enter Claude plan mode to explore the codebase and design the plan collaboratively.

- If the user invoked `/hive:plan <prompt>` with a prompt, use that as the starting point and begin exploring the codebase immediately.
- If the user invoked bare `/hive:plan`, ask: **"What do you want to build or change?"**

Then:
1. Explore the codebase to understand the project structure, patterns, and affected areas
2. Discuss the approach with the user — ask clarifying questions, propose alternatives
3. Iterate on the plan until the user is satisfied

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

### Step 6: Offer to Launch (BEFORE exiting plan mode)

**BEFORE calling `ExitPlanMode`**, present the user with an interactive launch flow using `AskUserQuestion`:

- **Question:** "Plan saved to `.hive/plans/plan-<id>.json`! Ready to launch a drone?"
- **Options:**
  - "Launch drone now" (Recommended) — after the user approves, exit plan mode and immediately invoke the `/hive:start` skill with the plan ID to launch the drone
  - "Save only" — exit plan mode and confirm the plan is saved, do NOT implement anything
- **Custom feedback (Other):** If the user types feedback, iterate on the plan based on their feedback, update the plan JSON, then re-present the same `AskUserQuestion` options

Only call `ExitPlanMode` AFTER the user has answered this question.

**CRITICAL — DO NOT IMPLEMENT THE PLAN YOURSELF:**
- When the user selects "Launch drone now", invoke `/hive:start` to launch a **drone** (a separate Claude Code instance in a worktree). You are NOT the drone. Do NOT write code, create files, or make changes yourself.
- When the user selects "Save only", just confirm the file is saved. Do NOT start implementing.
- When `ExitPlanMode` is approved by the user, that is approval to proceed with the launch/save action they already chose — it is NOT an instruction to implement the plan yourself.
- **You are the planner, not the implementer.** Your job ends after saving the plan JSON and optionally launching a drone via `/hive:start`.

This creates a tight loop: plan → review → feedback → plan → review → launch drone.

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
