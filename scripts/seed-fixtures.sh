#!/usr/bin/env bash
# seed-fixtures.sh — Create 3 fake drones so the WebUI renders all components.
# Usage: ./scripts/seed-fixtures.sh
set -euo pipefail

HIVE_DIR=".hive"
CLAUDE_DIR="$HOME/.claude"

NOW_RFC3339=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
HOUR_AGO=$(date -u -v-1H +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d '1 hour ago' +"%Y-%m-%dT%H:%M:%SZ")
TWO_HOURS_AGO=$(date -u -v-2H +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d '2 hours ago' +"%Y-%m-%dT%H:%M:%SZ")
THREE_HOURS_AGO=$(date -u -v-3H +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d '3 hours ago' +"%Y-%m-%dT%H:%M:%SZ")

# Epoch millis for task timestamps
NOW_MS=$(($(date +%s) * 1000))
EARLIER_MS=$(( NOW_MS - 3600000 ))
MUCH_EARLIER_MS=$(( NOW_MS - 7200000 ))

echo "Seeding fixture data for 3 drones..."

# =============================================================================
# Drone 1: auth-service — InProgress, 2/5 tasks done
# =============================================================================
DRONE="auth-service"
echo "  Creating $DRONE..."

mkdir -p "$HIVE_DIR/drones/$DRONE"
mkdir -p "$CLAUDE_DIR/tasks/$DRONE"
mkdir -p "$CLAUDE_DIR/teams/$DRONE"

cat > "$HIVE_DIR/drones/$DRONE/status.json" << 'EOF'
{
  "drone": "auth-service",
  "prd": "auth-service",
  "branch": "feat/auth-service",
  "worktree": ".hive/worktrees/auth-service",
  "local_mode": false,
  "execution_mode": "AgentTeam",
  "backend": "claude",
  "status": "in_progress",
  "current_task": "Implement JWT token refresh endpoint",
  "completed": ["Set up project structure", "Add user model and migrations"],
  "total": 5,
  "started": "HOUR_AGO",
  "updated": "NOW_RFC3339",
  "error_count": 0,
  "last_error": null,
  "lead_model": "claude-sonnet-4-5-20250929",
  "active_agents": {
    "worker-1": "Implementing JWT refresh",
    "worker-2": "Writing auth middleware"
  }
}
EOF
# Patch timestamps
sed -i '' "s|HOUR_AGO|$HOUR_AGO|g" "$HIVE_DIR/drones/$DRONE/status.json"
sed -i '' "s|NOW_RFC3339|$NOW_RFC3339|g" "$HIVE_DIR/drones/$DRONE/status.json"

# Tasks
cat > "$CLAUDE_DIR/tasks/$DRONE/1.json" << EOF
{
  "id": "1",
  "subject": "Set up project structure and dependencies",
  "description": "Initialize the auth service with Express.js, TypeScript, and required packages (jsonwebtoken, bcrypt, prisma).",
  "status": "completed",
  "owner": "worker-1",
  "activeForm": null,
  "blockedBy": [],
  "blocks": [],
  "metadata": null,
  "createdAt": $MUCH_EARLIER_MS,
  "updatedAt": $EARLIER_MS
}
EOF

cat > "$CLAUDE_DIR/tasks/$DRONE/2.json" << EOF
{
  "id": "2",
  "subject": "Add user model and database migrations",
  "description": "Create User model with Prisma schema including email, password hash, and role fields. Generate and run migrations.",
  "status": "completed",
  "owner": "worker-2",
  "activeForm": null,
  "blockedBy": [],
  "blocks": [],
  "metadata": null,
  "createdAt": $MUCH_EARLIER_MS,
  "updatedAt": $EARLIER_MS
}
EOF

cat > "$CLAUDE_DIR/tasks/$DRONE/3.json" << EOF
{
  "id": "3",
  "subject": "Implement JWT token refresh endpoint",
  "description": "Create POST /auth/refresh endpoint that validates refresh tokens and issues new access tokens.",
  "status": "in_progress",
  "owner": "worker-1",
  "activeForm": "Implementing JWT refresh endpoint",
  "blockedBy": [],
  "blocks": [],
  "metadata": null,
  "createdAt": $EARLIER_MS,
  "updatedAt": $NOW_MS
}
EOF

cat > "$CLAUDE_DIR/tasks/$DRONE/4.json" << EOF
{
  "id": "4",
  "subject": "Add authentication middleware",
  "description": "Create Express middleware that validates JWT tokens and attaches user context to requests.",
  "status": "in_progress",
  "owner": "worker-2",
  "activeForm": "Writing auth middleware",
  "blockedBy": [],
  "blocks": [],
  "metadata": null,
  "createdAt": $EARLIER_MS,
  "updatedAt": $NOW_MS
}
EOF

cat > "$CLAUDE_DIR/tasks/$DRONE/5.json" << EOF
{
  "id": "5",
  "subject": "Write integration tests for auth flow",
  "description": "Test signup, login, token refresh, and protected route access with supertest.",
  "status": "pending",
  "owner": null,
  "activeForm": null,
  "blockedBy": ["3", "4"],
  "blocks": [],
  "metadata": null,
  "createdAt": $EARLIER_MS,
  "updatedAt": $EARLIER_MS
}
EOF

# Team config
cat > "$CLAUDE_DIR/teams/$DRONE/config.json" << 'EOF'
{
  "name": "auth-service",
  "members": [
    {
      "name": "team-lead",
      "agentType": "team-lead",
      "model": "claude-sonnet-4-5-20250929",
      "cwd": ".hive/worktrees/auth-service"
    },
    {
      "name": "worker-1",
      "agentType": "general-purpose",
      "model": "claude-sonnet-4-5-20250929",
      "cwd": ".hive/worktrees/auth-service"
    },
    {
      "name": "worker-2",
      "agentType": "general-purpose",
      "model": "claude-sonnet-4-5-20250929",
      "cwd": ".hive/worktrees/auth-service"
    },
    {
      "name": "worker-3",
      "agentType": "general-purpose",
      "model": "claude-haiku-4-5-20251001",
      "cwd": ".hive/worktrees/auth-service"
    }
  ]
}
EOF

# Inbox messages
mkdir -p "$CLAUDE_DIR/teams/$DRONE/inboxes"

cat > "$CLAUDE_DIR/teams/$DRONE/inboxes/team-lead.json" << 'EOF'
[
  {
    "from": "worker-1",
    "text": "Project structure is set up. Express + TypeScript + Prisma all configured. Moving to the user model next.",
    "timestamp": "2026-02-12T17:15:00Z",
    "read": true
  },
  {
    "from": "worker-2",
    "text": "User model and migrations done. Schema includes email, password_hash, role, created_at, updated_at. Ready for the next task.",
    "timestamp": "2026-02-12T17:28:00Z",
    "read": true
  },
  {
    "from": "worker-1",
    "text": "Working on JWT refresh endpoint. Found an edge case with expired refresh tokens — adding a grace period of 30s. Will update when done.",
    "timestamp": "2026-02-12T17:45:00Z",
    "read": false
  },
  {
    "from": "worker-2",
    "text": "Auth middleware is looking good. Validates access tokens, attaches user to req.context. Should I also handle API key auth or just JWT?",
    "timestamp": "2026-02-12T17:52:00Z",
    "read": false
  }
]
EOF

cat > "$CLAUDE_DIR/teams/$DRONE/inboxes/worker-1.json" << 'EOF'
[
  {
    "from": "team-lead",
    "text": "Start with task 1 — set up the project structure. Use Express.js with TypeScript. Add jsonwebtoken, bcrypt, and prisma as dependencies.",
    "timestamp": "2026-02-12T17:10:30Z",
    "read": true
  },
  {
    "from": "team-lead",
    "text": "Good work on the setup. Now take task 3 — implement the JWT refresh endpoint. Build on the user model worker-2 created.",
    "timestamp": "2026-02-12T17:30:00Z",
    "read": true
  }
]
EOF

cat > "$CLAUDE_DIR/teams/$DRONE/inboxes/worker-2.json" << 'EOF'
[
  {
    "from": "team-lead",
    "text": "Take task 2 — create the User model with Prisma. Include email, password hash, and role fields. Run the migration when done.",
    "timestamp": "2026-02-12T17:11:00Z",
    "read": true
  },
  {
    "from": "team-lead",
    "text": "Nice. Move to task 4 — the auth middleware. Just JWT for now, we can add API key auth later.",
    "timestamp": "2026-02-12T17:53:00Z",
    "read": false
  }
]
EOF

# Activity log (cost data)
cat > "$HIVE_DIR/drones/$DRONE/activity.log" << 'EOF'
{"cost_usd":0.42,"usage":{"input_tokens":48000,"output_tokens":9200,"cache_read_input_tokens":22000,"cache_creation_input_tokens":5000}}
{"cost_usd":0.87,"usage":{"input_tokens":96000,"output_tokens":18400,"cache_read_input_tokens":54000,"cache_creation_input_tokens":9000}}
{"cost_usd":1.24,"usage":{"input_tokens":145234,"output_tokens":32456,"cache_read_input_tokens":89000,"cache_creation_input_tokens":12000}}
EOF

# =============================================================================
# Drone 2: frontend-revamp — Completed, 4/4 tasks done
# =============================================================================
DRONE="frontend-revamp"
echo "  Creating $DRONE..."

mkdir -p "$HIVE_DIR/drones/$DRONE"
mkdir -p "$CLAUDE_DIR/tasks/$DRONE"
mkdir -p "$CLAUDE_DIR/teams/$DRONE"

cat > "$HIVE_DIR/drones/$DRONE/status.json" << 'EOF'
{
  "drone": "frontend-revamp",
  "prd": "frontend-revamp",
  "branch": "feat/frontend-revamp",
  "worktree": ".hive/worktrees/frontend-revamp",
  "local_mode": false,
  "execution_mode": "AgentTeam",
  "backend": "claude",
  "status": "completed",
  "current_task": null,
  "completed": ["Migrate to Tailwind v4", "Refactor component library", "Add dark mode support", "Update E2E tests"],
  "total": 4,
  "started": "THREE_HOURS_AGO",
  "updated": "HOUR_AGO",
  "error_count": 0,
  "last_error": null,
  "lead_model": "claude-sonnet-4-5-20250929",
  "active_agents": {}
}
EOF
sed -i '' "s|THREE_HOURS_AGO|$THREE_HOURS_AGO|g" "$HIVE_DIR/drones/$DRONE/status.json"
sed -i '' "s|HOUR_AGO|$HOUR_AGO|g" "$HIVE_DIR/drones/$DRONE/status.json"

# Tasks — all completed
cat > "$CLAUDE_DIR/tasks/$DRONE/1.json" << EOF
{
  "id": "1",
  "subject": "Migrate to Tailwind v4",
  "description": "Upgrade from Tailwind CSS v3 to v4. Update config, replace deprecated utilities, and verify all components render correctly.",
  "status": "completed",
  "owner": "worker-1",
  "activeForm": null,
  "blockedBy": [],
  "blocks": [],
  "metadata": null,
  "createdAt": $MUCH_EARLIER_MS,
  "updatedAt": $EARLIER_MS
}
EOF

cat > "$CLAUDE_DIR/tasks/$DRONE/2.json" << EOF
{
  "id": "2",
  "subject": "Refactor component library to use composable patterns",
  "description": "Refactor Button, Card, Modal, and Input components to use the compound component pattern with Radix UI primitives.",
  "status": "completed",
  "owner": "worker-2",
  "activeForm": null,
  "blockedBy": [],
  "blocks": [],
  "metadata": null,
  "createdAt": $MUCH_EARLIER_MS,
  "updatedAt": $EARLIER_MS
}
EOF

cat > "$CLAUDE_DIR/tasks/$DRONE/3.json" << EOF
{
  "id": "3",
  "subject": "Add dark mode support",
  "description": "Implement system-aware dark mode using Tailwind dark: variants and a toggle in the settings page. Persist preference in localStorage.",
  "status": "completed",
  "owner": "worker-1",
  "activeForm": null,
  "blockedBy": ["1"],
  "blocks": [],
  "metadata": null,
  "createdAt": $EARLIER_MS,
  "updatedAt": $EARLIER_MS
}
EOF

cat > "$CLAUDE_DIR/tasks/$DRONE/4.json" << EOF
{
  "id": "4",
  "subject": "Update E2E tests for new component APIs",
  "description": "Update Playwright E2E tests to work with the new compound component APIs and dark mode toggle.",
  "status": "completed",
  "owner": "worker-3",
  "activeForm": null,
  "blockedBy": ["2", "3"],
  "blocks": [],
  "metadata": null,
  "createdAt": $EARLIER_MS,
  "updatedAt": $NOW_MS
}
EOF

# Team config
cat > "$CLAUDE_DIR/teams/$DRONE/config.json" << 'EOF'
{
  "name": "frontend-revamp",
  "members": [
    {
      "name": "team-lead",
      "agentType": "team-lead",
      "model": "claude-sonnet-4-5-20250929",
      "cwd": ".hive/worktrees/frontend-revamp"
    },
    {
      "name": "worker-1",
      "agentType": "general-purpose",
      "model": "claude-sonnet-4-5-20250929",
      "cwd": ".hive/worktrees/frontend-revamp"
    },
    {
      "name": "worker-2",
      "agentType": "general-purpose",
      "model": "claude-sonnet-4-5-20250929",
      "cwd": ".hive/worktrees/frontend-revamp"
    },
    {
      "name": "worker-3",
      "agentType": "general-purpose",
      "model": "claude-haiku-4-5-20251001",
      "cwd": ".hive/worktrees/frontend-revamp"
    }
  ]
}
EOF

# Inbox messages
mkdir -p "$CLAUDE_DIR/teams/$DRONE/inboxes"

cat > "$CLAUDE_DIR/teams/$DRONE/inboxes/team-lead.json" << 'EOF'
[
  {
    "from": "worker-1",
    "text": "Tailwind v4 migration complete. Replaced all deprecated utilities, updated config. All components render correctly.",
    "timestamp": "2026-02-12T15:20:00Z",
    "read": true
  },
  {
    "from": "worker-2",
    "text": "Component refactoring done. Button, Card, Modal, Input all use compound patterns now. Breaking change: Card.Header replaces CardHeader.",
    "timestamp": "2026-02-12T15:45:00Z",
    "read": true
  },
  {
    "from": "worker-1",
    "text": "Dark mode is live. Uses system preference by default with manual toggle. Preference persisted in localStorage.",
    "timestamp": "2026-02-12T16:10:00Z",
    "read": true
  },
  {
    "from": "worker-3",
    "text": "All E2E tests updated and passing. 47 tests, 0 failures. PR is ready for review.",
    "timestamp": "2026-02-12T16:30:00Z",
    "read": true
  }
]
EOF

cat > "$CLAUDE_DIR/teams/$DRONE/inboxes/worker-1.json" << 'EOF'
[
  {
    "from": "team-lead",
    "text": "Start with Tailwind v4 migration. After that, pick up dark mode support — it depends on your Tailwind work.",
    "timestamp": "2026-02-12T14:15:00Z",
    "read": true
  }
]
EOF

# Activity log
cat > "$HIVE_DIR/drones/$DRONE/activity.log" << 'EOF'
{"cost_usd":1.10,"usage":{"input_tokens":120000,"output_tokens":28000,"cache_read_input_tokens":65000,"cache_creation_input_tokens":15000}}
{"cost_usd":2.45,"usage":{"input_tokens":245000,"output_tokens":56000,"cache_read_input_tokens":130000,"cache_creation_input_tokens":28000}}
{"cost_usd":3.87,"usage":{"input_tokens":389000,"output_tokens":87000,"cache_read_input_tokens":210000,"cache_creation_input_tokens":42000}}
EOF

# =============================================================================
# Drone 3: db-migration — Stopped, 1/3 tasks done
# =============================================================================
DRONE="db-migration"
echo "  Creating $DRONE..."

mkdir -p "$HIVE_DIR/drones/$DRONE"
mkdir -p "$CLAUDE_DIR/tasks/$DRONE"
mkdir -p "$CLAUDE_DIR/teams/$DRONE"

cat > "$HIVE_DIR/drones/$DRONE/status.json" << 'EOF'
{
  "drone": "db-migration",
  "prd": "db-migration",
  "branch": "feat/db-migration",
  "worktree": ".hive/worktrees/db-migration",
  "local_mode": false,
  "execution_mode": "AgentTeam",
  "backend": "claude",
  "status": "stopped",
  "current_task": "Migrate user sessions to Redis",
  "completed": ["Create migration scripts for schema v2"],
  "total": 3,
  "started": "TWO_HOURS_AGO",
  "updated": "HOUR_AGO",
  "error_count": 1,
  "last_error": "Migrate user sessions to Redis",
  "lead_model": "claude-sonnet-4-5-20250929",
  "active_agents": {}
}
EOF
sed -i '' "s|TWO_HOURS_AGO|$TWO_HOURS_AGO|g" "$HIVE_DIR/drones/$DRONE/status.json"
sed -i '' "s|HOUR_AGO|$HOUR_AGO|g" "$HIVE_DIR/drones/$DRONE/status.json"

# Tasks — 1 done, 1 stuck in_progress, 1 pending
cat > "$CLAUDE_DIR/tasks/$DRONE/1.json" << EOF
{
  "id": "1",
  "subject": "Create migration scripts for schema v2",
  "description": "Write SQL migration scripts to add new columns, indexes, and constraints for the v2 schema.",
  "status": "completed",
  "owner": "worker-1",
  "activeForm": null,
  "blockedBy": [],
  "blocks": [],
  "metadata": null,
  "createdAt": $MUCH_EARLIER_MS,
  "updatedAt": $EARLIER_MS
}
EOF

cat > "$CLAUDE_DIR/tasks/$DRONE/2.json" << EOF
{
  "id": "2",
  "subject": "Migrate user sessions to Redis",
  "description": "Move session storage from PostgreSQL to Redis. Update session middleware and add connection pooling.",
  "status": "in_progress",
  "owner": "worker-1",
  "activeForm": "Migrating session storage to Redis",
  "blockedBy": [],
  "blocks": [],
  "metadata": null,
  "createdAt": $EARLIER_MS,
  "updatedAt": $EARLIER_MS
}
EOF

cat > "$CLAUDE_DIR/tasks/$DRONE/3.json" << EOF
{
  "id": "3",
  "subject": "Add rollback procedures and verification",
  "description": "Create rollback scripts for each migration step and a verification suite to confirm data integrity post-migration.",
  "status": "pending",
  "owner": null,
  "activeForm": null,
  "blockedBy": ["2"],
  "blocks": [],
  "metadata": null,
  "createdAt": $EARLIER_MS,
  "updatedAt": $EARLIER_MS
}
EOF

# Team config (2 workers)
cat > "$CLAUDE_DIR/teams/$DRONE/config.json" << 'EOF'
{
  "name": "db-migration",
  "members": [
    {
      "name": "team-lead",
      "agentType": "team-lead",
      "model": "claude-sonnet-4-5-20250929",
      "cwd": ".hive/worktrees/db-migration"
    },
    {
      "name": "worker-1",
      "agentType": "general-purpose",
      "model": "claude-sonnet-4-5-20250929",
      "cwd": ".hive/worktrees/db-migration"
    },
    {
      "name": "worker-2",
      "agentType": "general-purpose",
      "model": "claude-haiku-4-5-20251001",
      "cwd": ".hive/worktrees/db-migration"
    }
  ]
}
EOF

# Inbox messages
mkdir -p "$CLAUDE_DIR/teams/$DRONE/inboxes"

cat > "$CLAUDE_DIR/teams/$DRONE/inboxes/team-lead.json" << 'EOF'
[
  {
    "from": "worker-1",
    "text": "Migration scripts for schema v2 are done. Added columns, indexes, and constraints. Starting on the Redis session migration.",
    "timestamp": "2026-02-12T16:25:00Z",
    "read": true
  },
  {
    "from": "worker-1",
    "text": "Hit an issue connecting to Redis. Connection pooling config seems wrong. Getting ECONNREFUSED on port 6379.",
    "timestamp": "2026-02-12T16:48:00Z",
    "read": true
  }
]
EOF

cat > "$CLAUDE_DIR/teams/$DRONE/inboxes/worker-1.json" << 'EOF'
[
  {
    "from": "team-lead",
    "text": "Start with the SQL migration scripts for schema v2. Once done, move to Redis session migration.",
    "timestamp": "2026-02-12T16:12:00Z",
    "read": true
  },
  {
    "from": "team-lead",
    "text": "Check if Redis is running locally. Try docker compose up redis if it's containerized.",
    "timestamp": "2026-02-12T16:50:00Z",
    "read": false
  }
]
EOF

# Activity log
cat > "$HIVE_DIR/drones/$DRONE/activity.log" << 'EOF'
{"cost_usd":0.18,"usage":{"input_tokens":19000,"output_tokens":4200,"cache_read_input_tokens":8000,"cache_creation_input_tokens":2000}}
{"cost_usd":0.52,"usage":{"input_tokens":54000,"output_tokens":12800,"cache_read_input_tokens":28000,"cache_creation_input_tokens":6000}}
EOF

echo ""
echo "Done! Fixture data created for 3 drones:"
echo "  - auth-service    (in_progress, 2/5 tasks, \$1.24)"
echo "  - frontend-revamp (completed,   4/4 tasks, \$3.87)"
echo "  - db-migration    (stopped,     1/3 tasks, \$0.52)"
echo ""
echo "Run 'cargo run -- monitor --web' and open http://localhost:3333"
