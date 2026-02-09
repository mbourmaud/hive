# Hive Plan Guide

Complete guide to creating effective plans for Hive drones in v3.0.0.

## Table of Contents

- [Overview](#overview)
- [Plan Structure](#plan-structure)
- [Writing Effective Plans](#writing-effective-plans)
- [Plan Examples](#plan-examples)
- [Best Practices](#best-practices)
- [Tips for Success](#tips-for-success)

## Overview

In Hive v3.0.0, plans are **freeform markdown documents** that guide autonomous drones through implementation tasks. Unlike the previous PRD system with rigid stories and Definition of Done cards, plans are flexible and conversational.

**Key Principles**:
- Plans are created collaboratively with Claude using `/hive:plan`
- Simple tasks need simple plans, complex tasks can have more detail
- Plans are **markdown**, not structured data
- The drone reads the plan and executes it autonomously

## Plan Structure

### Plan JSON Schema

```typescript
interface Plan {
  id: string;                    // kebab-case identifier (e.g., "add-jwt-auth")
  title: string;                 // Human-readable title
  description?: string;          // Brief summary (1 sentence)
  version?: string;              // e.g., "1.0.0"
  created_at?: string;           // ISO timestamp
  target_platforms?: string[];   // Optional platform targets
  target_branch?: string;        // Git branch (default: hive/{id})
  base_branch?: string;          // Base branch for worktree (default: auto-detect)
  plan: string;                  // Freeform markdown plan
}
```

### Example Plan File

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

## Writing Effective Plans

The `plan` field is **freeform markdown**. There's no required structure, but here are recommended sections that help drones succeed:

### Recommended Sections

#### 1. Goal (Essential)
What you're trying to achieve in 1-2 sentences.

```markdown
## Goal
Add JWT-based authentication to protect all API endpoints from unauthorized access.
```

#### 2. Requirements (Essential)
Key things that must be true when the work is done.

```markdown
## Requirements
- All /api/* routes require a valid JWT token
- Tokens are verified against JWT_SECRET from .env
- Unauthorized requests return 401 with clear error message
- Existing tests continue to pass
```

#### 3. Approach (Recommended)
High-level steps and architectural decisions.

```markdown
## Approach
1. Create auth middleware in src/middleware/auth.ts
2. Implement JWT verification using jsonwebtoken library
3. Add middleware to all API route handlers in src/routes/
4. Update .env.example with JWT_SECRET placeholder
5. Write comprehensive tests
```

#### 4. Files Affected (Recommended)
Which parts of the codebase will change.

```markdown
## Files Affected
- src/middleware/auth.ts (new)
- src/routes/api.ts (modify - add middleware)
- src/routes/users.ts (modify - add middleware)
- .env.example (modify - add JWT_SECRET)
- src/__tests__/auth.test.ts (new)
```

#### 5. Testing Strategy (Recommended)
How to verify the work is correct.

```markdown
## Testing
- Unit tests for auth middleware (valid token, invalid token, missing token)
- Integration tests for each protected route (401 without auth, 200 with auth)
- Update existing tests to include auth headers
- Verify build passes: `npm run build`
- Verify test suite passes: `npm test`
```

#### 6. Context (Optional but Helpful)
Additional context, constraints, or architectural notes.

```markdown
## Context
- Follow existing middleware pattern in src/middleware/
- Use jsonwebtoken library (already in package.json)
- JWT_SECRET should be loaded from environment variables
- Maintain backward compatibility for public routes (/health, /docs)
```

#### 7. PR/MR Story (Optional)
If the plan includes creating a pull request.

```markdown
## PR/MR Story
Once implementation is complete:
1. Create a pull request to main branch
2. Title: "feat: add JWT authentication to API endpoints"
3. Ensure CI passes (tests, build, linting)
4. Request review if needed
```

### Freeform Structure

You can use whatever structure makes sense for your task. Here's a minimal example:

```markdown
Fix the authentication bug in login endpoint.

The issue is in src/routes/auth.ts line 42 - password comparison is using == instead of bcrypt.compare().

Steps:
1. Replace password check with bcrypt.compare()
2. Add test case for invalid password
3. Verify existing tests still pass

That's it!
```

## Plan Examples

### Example 1: Simple Bug Fix

```json
{
  "id": "fix-login-password-check",
  "title": "Fix password comparison in login",
  "description": "Replace incorrect password check with bcrypt comparison",
  "plan": "## Goal\nFix authentication bug where passwords are compared with == instead of bcrypt.compare()\n\n## Changes\n- File: src/routes/auth.ts, line 42\n- Replace `if (password == user.passwordHash)` with `if (await bcrypt.compare(password, user.passwordHash))`\n\n## Testing\n- Add test case for invalid password scenario\n- Verify existing login tests still pass\n- Run: `npm test src/__tests__/auth.test.ts`"
}
```

### Example 2: Medium Feature

```json
{
  "id": "add-user-avatar-upload",
  "title": "Add User Avatar Upload Feature",
  "description": "Allow users to upload and display profile avatars",
  "plan": "## Goal\nAdd avatar upload functionality so users can customize their profiles with images.\n\n## Requirements\n- Users can upload images (PNG, JPG, max 5MB)\n- Uploaded images are resized to 200x200px\n- Avatar is displayed on profile page and next to posts\n- Old avatars are deleted when new ones are uploaded\n\n## Approach\n1. Create avatar upload endpoint: POST /api/users/:id/avatar\n2. Use multer middleware for file upload handling\n3. Use sharp library for image resizing\n4. Store avatars in /public/avatars/ directory\n5. Update User model to include avatarUrl field\n6. Update profile UI to show avatar and upload button\n\n## Files Affected\n- src/routes/users.ts (add avatar upload endpoint)\n- src/models/User.ts (add avatarUrl field)\n- src/utils/imageProcessor.ts (new - handle resize/save)\n- public/avatars/ (new directory)\n- src/components/Profile.tsx (add avatar display + upload)\n- src/__tests__/avatar.test.ts (new tests)\n\n## Testing\n- Unit tests for image processing (resize, validation)\n- Integration tests for upload endpoint (success, file too large, wrong format)\n- E2E test for upload flow in UI\n- Verify avatars display correctly"
}
```

### Example 3: Complex Feature with Architecture

```json
{
  "id": "real-time-notifications",
  "title": "Real-time Notification System",
  "description": "Implement WebSocket-based real-time notifications for user events",
  "plan": "## Goal\nImplement real-time notification system using WebSockets so users receive instant updates for mentions, likes, and comments.\n\n## Requirements\n- Users receive notifications in real-time (no polling)\n- Notifications persist across page refreshes\n- Unread count badge updates automatically\n- System handles 1000+ concurrent connections\n- Graceful fallback if WebSocket connection fails\n\n## Architecture\n- WebSocket server using socket.io\n- Redis pub/sub for multi-server scalability\n- PostgreSQL for notification persistence\n- Client-side notification queue for offline handling\n\n## Approach\n\n### Backend (Phase 1)\n1. Set up socket.io server in src/ws/server.ts\n2. Create Notification model (id, userId, type, content, read, createdAt)\n3. Create NotificationService for business logic\n4. Integrate Redis pub/sub for horizontal scaling\n5. Add WebSocket authentication middleware\n\n### Backend (Phase 2)\n6. Create notification triggers in existing routes:\n   - POST /api/posts/:id/like → notify post author\n   - POST /api/posts/:id/comments → notify post author\n   - Mention detection → notify mentioned users\n7. Add REST endpoints for notification history:\n   - GET /api/notifications (list)\n   - PATCH /api/notifications/:id/read (mark read)\n\n### Frontend (Phase 3)\n8. Create WebSocket client in src/services/socket.ts\n9. Create NotificationContext for state management\n10. Add notification bell icon with unread badge\n11. Add notification dropdown panel\n12. Implement offline queue (store in localStorage)\n\n### Testing (Phase 4)\n13. Unit tests for NotificationService\n14. Integration tests for WebSocket events\n15. Load tests for 1000 concurrent connections\n16. E2E tests for notification flow\n\n## Files Affected\n\n### Backend\n- src/ws/server.ts (new)\n- src/ws/auth.ts (new - WebSocket auth middleware)\n- src/models/Notification.ts (new)\n- src/services/NotificationService.ts (new)\n- src/routes/notifications.ts (new - REST API)\n- src/routes/posts.ts (modify - trigger notifications)\n- src/routes/comments.ts (modify - trigger notifications)\n\n### Frontend\n- src/services/socket.ts (new)\n- src/context/NotificationContext.tsx (new)\n- src/components/NotificationBell.tsx (new)\n- src/components/NotificationPanel.tsx (new)\n- src/components/Layout.tsx (modify - add notification bell)\n\n### Infrastructure\n- docker-compose.yml (add Redis service)\n- .env.example (add REDIS_URL, WS_PORT)\n\n## Testing Strategy\n\n### Unit Tests\n- NotificationService.create(), markRead(), getUnread()\n- Mention detection logic\n- WebSocket authentication\n\n### Integration Tests\n- Full notification flow (trigger → Redis → WebSocket → client)\n- REST API endpoints\n- Multiple users receiving same notification\n\n### Load Tests\n- 1000 concurrent WebSocket connections\n- Message throughput under load\n- Redis pub/sub performance\n\n### E2E Tests (Playwright)\n- User A likes post → User B receives notification\n- Notification count updates in real-time\n- Clicking notification marks as read\n- Offline queue works when reconnecting\n\n## Dependencies\n- socket.io (WebSocket library)\n- redis (pub/sub for scaling)\n- ioredis (Redis client)\n- @socket.io/redis-adapter (Redis adapter for socket.io)\n\n## Rollout Plan\n1. Deploy with feature flag disabled\n2. Enable for 10% of users (beta)\n3. Monitor WebSocket connection stability\n4. Monitor Redis pub/sub performance\n5. Gradual rollout to 100%\n\n## Rollback Plan\nIf issues occur:\n1. Set feature flag to false\n2. Users fall back to polling every 30s\n3. WebSocket server can be stopped independently\n\n## PR/MR Story\nOnce complete:\n1. Create PR with title: \"feat: real-time notification system with WebSockets\"\n2. Include load test results in PR description\n3. Ensure CI passes (tests, build, linting)\n4. Deploy to staging for QA testing\n5. Monitor staging for 24h before production deploy"
}
```

## Best Practices

### 1. Match Complexity to Task

**Simple tasks = simple plans**
```markdown
Fix typo in login error message.

File: src/routes/auth.ts, line 23
Change "Pasword incorrect" to "Password incorrect"
```

**Complex tasks = detailed plans**
Use all recommended sections (Goal, Requirements, Approach, Testing, etc.)

### 2. Be Specific About Files and Locations

**Good**:
```markdown
## Files Affected
- src/middleware/auth.ts (new - create JWT middleware)
- src/routes/api.ts, line 15 (modify - add middleware to router)
- .env.example (modify - add JWT_SECRET=your-secret-here)
```

**Bad**:
```markdown
## Files Affected
- Some auth files
- Config files
```

### 3. Include Testing Strategy

**Good**:
```markdown
## Testing
- Unit test: middleware with valid token (should call next())
- Unit test: middleware with invalid token (should return 401)
- Unit test: middleware with missing token (should return 401)
- Integration test: protected route without auth (expect 401)
- Integration test: protected route with auth (expect 200)
- Run full test suite: `npm test`
- Verify build: `npm run build`
```

**Bad**:
```markdown
## Testing
- Test it
```

### 4. Provide Architectural Context

**Good**:
```markdown
## Context
- Follow existing middleware pattern in src/middleware/
- Use jsonwebtoken library (already installed)
- Middleware should be added BEFORE route handlers
- Don't break public routes (/health, /docs)
- JWT_SECRET must come from environment variables (never hardcode)
```

**Bad**:
```markdown
## Context
- Use good patterns
```

### 5. Consider Error Scenarios

**Good**:
```markdown
## Edge Cases
- Handle expired tokens (return 401 with "Token expired" message)
- Handle malformed tokens (return 401 with "Invalid token" message)
- Handle missing Authorization header (return 401 with "No token provided")
- Public routes should bypass auth (check route before applying middleware)
```

### 6. Include PR/MR Guidance (if applicable)

**Good**:
```markdown
## PR/MR Story
1. Run full test suite and build locally
2. Create PR to main branch
3. Title: "feat: add JWT authentication middleware"
4. Include testing details in PR description
5. Ensure CI passes (GitHub Actions)
6. Request review from @security-team
```

## Tips for Success

### 1. Use `/hive:plan` for Collaborative Planning

The best plans come from conversation with Claude:
```
You: I want to add OAuth authentication
Claude: Let me explore your codebase...
        [explores existing auth, patterns, tests]
        I see you're using passport.js. Should we add OAuth providers
        to the existing passport setup or create a separate flow?
You: Use the existing passport setup
Claude: Great! Here's the plan...
        [creates detailed plan based on actual codebase]
```

### 2. Start with Goal and Requirements

Begin every plan with what you're trying to achieve and what must be true when done.

### 3. Reference Existing Code Patterns

```markdown
## Context
- Follow the same pattern as existing email auth in src/auth/email.ts
- Use the same error handling as src/middleware/errorHandler.ts
- Tests should match the style in src/__tests__/auth.test.ts
```

### 4. Break Down Large Tasks

For very large features, consider creating multiple plans:
- `oauth-backend` (backend OAuth implementation)
- `oauth-frontend` (frontend OAuth UI)
- `oauth-testing` (comprehensive E2E tests)

### 5. Include Commands to Run

Make it easy for the drone to verify success:

```markdown
## Verification
Run these commands to verify completion:
- `npm test src/__tests__/auth.test.ts` (auth tests pass)
- `npm run build` (build succeeds)
- `npm run lint` (no linting errors)
- `grep -r "JWT" src/middleware/` (verify JWT middleware exists)
```

### 6. Think About Rollback

For risky changes:

```markdown
## Rollback Plan
If this breaks production:
1. Revert commit: git revert <commit-hash>
2. Redeploy previous version
3. Feature flag "oauth_enabled" can be set to false in .env
```

## Common Mistakes to Avoid

### ❌ Being Too Vague

```markdown
## Goal
Make auth better
```

### ✅ Being Specific

```markdown
## Goal
Add JWT authentication to replace the current session-based auth,
improving security and enabling stateless API access.
```

---

### ❌ No Testing Guidance

```markdown
## Approach
1. Write the code
2. Test it
```

### ✅ Clear Testing Strategy

```markdown
## Testing
- Unit tests: src/__tests__/jwt.test.ts
  - Test token generation
  - Test token verification (valid, expired, malformed)
- Integration tests: src/__tests__/api/auth.test.ts
  - Test protected endpoints with/without auth
- Run: `npm test`
```

---

### ❌ No File Context

```markdown
## Approach
Add middleware and update routes
```

### ✅ Specific File Locations

```markdown
## Files Affected
- src/middleware/jwt.ts (new - create middleware)
- src/routes/api.ts, line 10 (add middleware to router)
- src/utils/jwt.ts (new - token generation/verification helpers)
```

## Plan File Naming

Plans are saved to `.hive/plans/plan-<id>.json`:

- `plan-add-jwt-auth.json`
- `plan-fix-login-bug.json`
- `plan-user-avatar-upload.json`

The `id` field becomes part of the filename and default branch name:
- ID: `add-jwt-auth`
- File: `.hive/plans/plan-add-jwt-auth.json`
- Branch: `hive/add-jwt-auth` (default)

## Launching a Drone

Once your plan is created:

```bash
# Launch a drone with the plan
hive start add-jwt-auth

# Monitor progress
hive monitor

# View logs
hive logs add-jwt-auth
```

## Updating a Plan

Plans are created collaboratively, so iterate with Claude before launching:

```
You: /hive:plan
Claude: What do you want to build?
You: Add OAuth
Claude: [explores codebase, asks questions, drafts plan]
You: Can you add more detail about testing?
Claude: [updates plan with comprehensive testing section]
You: Perfect!
Claude: Saved to .hive/plans/plan-add-oauth.json
```

## See Also

- [Hive Documentation](README.md)
- [CLAUDE.md](CLAUDE.md) - Project instructions
- [Example Plans](.hive/plans/) - Real plan examples
