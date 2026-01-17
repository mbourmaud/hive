# Hive PRD - Generate a PRD for Drones

Generate a PRD (Product Requirements Document) JSON file optimized for Hive drones.

## What this does

Creates a PRD JSON file in `.hive/prds/` with well-defined stories that a drone can execute autonomously.

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

Use Glob, Grep, and Read tools to gather context.

### Step 4: PRD Metadata

Ask the user for:
- **PRD ID** - Short kebab-case identifier (e.g., `security-api-protection`)
- **Title** - Human-readable title
- **Description** - Brief summary of the overall goal

### Step 5: Break Down into Stories

Create atomic, well-scoped stories. Each story should:
- Be completable in 1-5 files
- Have clear acceptance criteria
- Be independent (can be done without other stories, ideally)
- Have a unique ID with prefix (e.g., `SEC-001`, `FEAT-001`)

### Step 6: Review with User

Present the stories in a clear format:
```
PRD: security-api-protection
"Secure API Routes" - 10 stories

SEC-001: Protect /api/accounts/*
  Files: src/app/api/accounts/route.ts
  Criteria: GET returns 401 if not authenticated

SEC-002: Protect /api/users/*
  Files: src/app/api/users/route.ts
  Criteria: All endpoints require auth

...
```

Ask: **"Does this look good? Want to add, remove, or modify any stories?"**

### Step 7: Generate PRD File

Write the JSON file to `.hive/prds/prd-<id>.json`:

```json
{
  "id": "security-api-protection",
  "title": "Secure API Routes",
  "description": "Add authentication to all API routes",
  "created": "2024-01-17T12:00:00Z",
  "stories": [
    {
      "id": "SEC-001",
      "title": "Protect /api/accounts/*",
      "description": "Add requireAuth() middleware to account routes",
      "acceptance_criteria": [
        "GET /api/accounts returns 401 if not authenticated",
        "POST /api/accounts returns 401 if not authenticated",
        "Authenticated requests work normally"
      ],
      "files": [
        "src/app/api/accounts/route.ts"
      ],
      "dependencies": []
    }
  ]
}
```

### Step 8: Next Steps

Tell the user:
```
PRD created: .hive/prds/prd-security-api-protection.json

To launch a drone on this PRD:
  /hive:start

Or via CLI:
  hive start --prd .hive/prds/prd-security-api-protection.json
```

## PRD JSON Schema

```typescript
interface PRD {
  id: string;              // kebab-case identifier
  title: string;           // Human-readable title
  description: string;     // Overall goal
  created: string;         // ISO timestamp
  stories: Story[];
}

interface Story {
  id: string;              // Unique ID (e.g., "SEC-001")
  title: string;           // Short title
  description: string;     // What to implement
  acceptance_criteria: string[];  // How to verify it's done
  files: string[];         // Files to modify/create
  dependencies?: string[]; // Other story IDs this depends on
}
```

## Tips for Good Stories

1. **Atomic** - One logical change per story
2. **Testable** - Clear criteria to verify completion
3. **Scoped** - List specific files, not "update the codebase"
4. **Ordered** - Put foundational stories first if there are dependencies
5. **Independent** - Minimize dependencies between stories when possible

## File Location

PRDs are stored in `.hive/prds/` which is:
- Gitignored (not committed)
- Shared via symlink with drone worktrees
- Accessible by both queen and drones

## Example PRDs

### Small PRD (3 stories)
```json
{
  "id": "add-dark-mode",
  "title": "Dark Mode Support",
  "stories": [
    {"id": "DM-001", "title": "Add theme context", ...},
    {"id": "DM-002", "title": "Create toggle component", ...},
    {"id": "DM-003", "title": "Update global styles", ...}
  ]
}
```

### Large PRD (10+ stories)
Consider splitting into multiple PRDs that can run in parallel on different drones.
