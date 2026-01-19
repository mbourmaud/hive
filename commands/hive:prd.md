# Hive PRD - Generate a PRD for Drones

Generate a PRD (Product Requirements Document) JSON file optimized for Hive drones.

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

Use Glob, Grep, and Read tools to gather context.

### Step 4: PRD Metadata

Ask the user for:
- **PRD ID** - Short kebab-case identifier (e.g., `security-api-protection`)
- **Title** - Human-readable title
- **Description** - Brief summary of the overall goal

### Step 5: Break Down into Stories with Interactive DoD

For **EACH story**, follow this process:

#### 5a. Propose the story
```
📋 Story: SEC-001 - Protect /api/accounts/*

Description: Add requireAuth() middleware to account routes

Files concernés:
- src/app/api/accounts/route.ts
```

#### 5b. Propose Definition of Done and ASK for validation
```
🎯 Definition of Done proposée:

Cette story est TERMINÉE quand:
1. Le middleware requireAuth() est ajouté aux routes GET et POST
2. Un test vérifie que GET /api/accounts retourne 401 sans auth
3. Un test vérifie que GET /api/accounts retourne 200 avec auth

Commandes de vérification (le drone DOIT les exécuter):
- `grep -r "requireAuth" src/app/api/accounts/` → doit matcher
- `pnpm test --filter=accounts` → doit passer

❓ Est-ce que cette Definition of Done est correcte ?
   - Manque-t-il quelque chose ?
   - Faut-il ajouter d'autres vérifications ?
```

#### 5c. Iterate until user validates
The user might say:
- "Il faut aussi logger les tentatives non authentifiées"
- "Ajoute un test e2e"
- "OK c'est bon"

Update the DoD based on feedback, then move to next story.

### Step 6: Review Complete PRD

Once all stories have validated DoDs, present the full PRD:
```
PRD: security-api-protection
"Secure API Routes" - 3 stories

SEC-001: Protect /api/accounts/*
  ✅ DoD validée

SEC-002: Protect /api/users/*
  ✅ DoD validée

SEC-003: Add auth logging
  ✅ DoD validée
```

Ask: **"Le PRD complet te convient ? Je génère le fichier ?"**

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
      "definition_of_done": [
        "Le middleware requireAuth() est ajouté aux routes GET et POST",
        "Un test vérifie que GET /api/accounts retourne 401 sans auth",
        "Un test vérifie que GET /api/accounts retourne 200 avec auth"
      ],
      "verification_commands": [
        {
          "command": "grep -r 'requireAuth' src/app/api/accounts/",
          "expected": "Au moins un match"
        },
        {
          "command": "pnpm test --filter=accounts",
          "expected": "Exit code 0 (tests passent)"
        }
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
  definition_of_done: string[];      // Clear, validated DoD statements
  verification_commands: {           // Commands drone MUST run to prove completion
    command: string;
    expected: string;
  }[];
  files: string[];         // Files to modify/create
  dependencies?: string[]; // Other story IDs this depends on
}
```

## Definition of Done Guidelines

A good DoD must be:

1. **Vérifiable** - Peut être prouvé par une commande ou un check
2. **Spécifique** - Pas de "ça marche bien", mais "retourne 200 avec body {x: y}"
3. **Complet** - Inclut tests, commits, et toute action requise
4. **Validé** - L'utilisateur a explicitement confirmé

### Examples of GOOD DoD items:
- "Le fichier `src/auth.ts` contient la fonction `validateToken()`"
- "Les tests passent: `pnpm nx test plato --testPathPattern=auth`"
- "Un commit est créé avec le message `feat(auth): add token validation`"
- "La route GET /api/users retourne 401 sans header Authorization"

### Examples of BAD DoD items:
- "L'authentification fonctionne" (trop vague)
- "Le code est propre" (subjectif)
- "Tout est testé" (pas spécifique)

## Verification Commands

Each story SHOULD have verification commands that the drone will execute to PROVE the story is complete. These are not optional - they are mandatory checks.

Types of verification:
- **File existence**: `test -f src/auth.ts && echo "OK"`
- **Code presence**: `grep -q "functionName" src/file.ts && echo "OK"`
- **Tests pass**: `pnpm test --filter=module`
- **Build succeeds**: `pnpm build`
- **Git status**: `git diff --name-only` to verify files changed
- **API check**: `curl -s localhost:3000/api/health | jq .status`

## File Location

PRDs are stored in `.hive/prds/` which is:
- Gitignored (not committed)
- Shared via symlink with drone worktrees
- Accessible by both queen and drones

## Interactive Clarification Questions

When defining DoD, ask clarifying questions like:

- "Cette story nécessite-t-elle un test unitaire, d'intégration, ou les deux ?"
- "Faut-il un commit séparé ou ça peut être groupé ?"
- "Y a-t-il des effets de bord à vérifier ?"
- "Quelle commande permet de vérifier que c'est fait ?"
- "Est-ce que le drone doit aussi mettre à jour la doc ?"

## Important Rules

1. **NEVER generate a story without user validation of its DoD**
2. **ALWAYS include at least one verification command per story**
3. **ASK questions when the DoD is ambiguous**
4. **ITERATE until the user says "OK" or "c'est bon"**
