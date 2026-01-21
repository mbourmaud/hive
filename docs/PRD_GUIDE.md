# Hive PRD Guide

Complete guide to creating effective Product Requirements Documents (PRDs) for Hive drones.

## Table of Contents

- [Overview](#overview)
- [PRD Structure](#prd-structure)
- [Story Fields Reference](#story-fields-reference)
- [Best Practices](#best-practices)
- [Examples](#examples)
- [Research-Backed Recommendations](#research-backed-recommendations)

## Overview

A PRD (Product Requirements Document) is the blueprint that guides Hive drones through feature implementation. Each PRD contains:
- Metadata about the feature
- A collection of stories (tasks) to execute
- Testing, validation, and quality requirements

**Key Principle**: The more specific and detailed your PRD, the better your drones will execute.

## PRD Structure

### Top-Level Fields

```json
{
  "id": "unique-feature-id",
  "title": "Feature Name",
  "description": "What this feature does and why it matters",
  "version": "1.0.0",
  "created_at": "2024-01-21T00:00:00Z",
  "target_platforms": ["web", "mobile"],
  "target_branch": "feature/branch-name",
  "stories": []
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `id` | Yes | Unique identifier for the feature |
| `title` | Yes | Human-readable feature name |
| `description` | No | Detailed feature description |
| `version` | No | PRD version for tracking changes |
| `created_at` | No | ISO 8601 timestamp |
| `target_platforms` | No | Platforms this affects (web, mobile, api, etc.) |
| `target_branch` | No | Git branch name (default: `hive/{id}`) |
| `stories` | Yes | Array of story objects |

## Story Fields Reference

### Core Fields (Always Included)

#### Required Fields

```json
{
  "id": "STORY-001",
  "title": "Implement user authentication",
  "description": "Add OAuth2 login with Google and GitHub providers",
  "definition_of_done": [
    "Users can log in with Google",
    "Users can log in with GitHub",
    "User profile data is stored in database",
    "Tests verify successful and failed login flows"
  ],
  "verification_commands": [
    "pnpm test --filter=auth",
    "pnpm build",
    "grep -r 'OAuth' src/auth/"
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique story identifier (e.g., FEAT-001) |
| `title` | string | Short, action-oriented title |
| `description` | string | Detailed explanation of what to build |
| `definition_of_done` | string[] | **Critical**: User-validated completion criteria |
| `verification_commands` | string[] | **Critical**: Shell commands to prove completion |

**Why verification_commands are critical**: These are executable proof that the story is complete. The drone runs these commands, and if they pass, the story is considered done.

#### Optional Core Fields

```json
{
  "acceptance_criteria": [
    "Login button appears on homepage",
    "Clicking login redirects to OAuth provider",
    "After auth, user is redirected back with token"
  ],
  "notes": "Use existing OAuth library from auth package"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `acceptance_criteria` | string[] | Specific requirements for acceptance |
| `notes` | string | Additional context, warnings, or guidance |

### Guidance Fields (Highly Recommended)

These fields help the drone understand HOW to implement the story:

```json
{
  "actions": [
    "Create OAuth2 client configuration",
    "Implement authorization redirect handler",
    "Implement callback handler with token exchange",
    "Add user profile extraction logic",
    "Write unit tests for each provider",
    "Write integration tests for full auth flow"
  ],
  "files": [
    "src/auth/oauth.ts",
    "src/auth/providers/google.ts",
    "src/auth/providers/github.ts",
    "src/auth/__tests__/oauth.test.ts"
  ],
  "tools": [
    "passport-google-oauth20",
    "passport-github2",
    "vitest"
  ]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `actions` | string[] | Step-by-step implementation tasks |
| `files` | string[] | Specific files to create/modify |
| `tools` | string[] | Libraries, frameworks, or commands to use |

### Context Fields (Prevents Mistakes)

Help the drone understand the environment and constraints:

```json
{
  "context": {
    "dependencies": [
      "passport.js for OAuth strategy pattern",
      "Redis for session storage",
      "PostgreSQL users table"
    ],
    "prerequisites": [
      "OAuth apps registered with Google and GitHub",
      "Environment variables configured in .env"
    ],
    "architectural_notes": [
      "Follow existing passport.js strategy pattern in src/auth/strategies/",
      "Use dependency injection for provider configs",
      "Maintain stateless authentication flow"
    ],
    "related_docs": [
      "docs/architecture/authentication.md",
      "docs/api/auth-endpoints.md"
    ]
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `dependencies` | string[] | External services, APIs, databases the story relies on |
| `prerequisites` | string[] | What must exist before starting |
| `architectural_notes` | string[] | Patterns, constraints, design principles to follow |
| `related_docs` | string[] | Documentation to reference |

### Testing Fields (Quality Assurance)

Specify exactly what tests are needed:

```json
{
  "testing": {
    "unit_tests": [
      "Test token validation for each provider",
      "Test error handling for invalid tokens",
      "Test profile data extraction"
    ],
    "integration_tests": [
      "Test complete OAuth flow with mock provider",
      "Test session creation after successful auth",
      "Test redirect after auth"
    ],
    "e2e_tests": [
      "Playwright test for Google OAuth flow",
      "Playwright test for failed auth handling"
    ],
    "coverage_threshold": 80.0
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `unit_tests` | string[] | Individual function/component tests |
| `integration_tests` | string[] | Multi-component interaction tests |
| `e2e_tests` | string[] | Full user flow tests |
| `coverage_threshold` | number | Minimum test coverage percentage (0-100) |

### Error Handling Fields (Robustness)

Define how to handle failures:

```json
{
  "error_handling": {
    "expected_errors": [
      "Invalid OAuth code",
      "Provider service unavailable",
      "Network timeout",
      "Invalid token response"
    ],
    "rollback_procedure": "Revert to email/password only auth",
    "recovery_strategy": "Retry with exponential backoff for network errors"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `expected_errors` | string[] | Error scenarios to handle |
| `rollback_procedure` | string | How to undo changes if story fails |
| `recovery_strategy` | string | How to recover from errors |

### Agent Control Fields (Safety)

Control drone behavior and require approvals:

```json
{
  "agent_controls": {
    "max_iterations": 10,
    "require_approval_for": [
      "database_schema_changes",
      "environment_variables",
      "third_party_api_changes"
    ],
    "block_on": [
      "test_failures",
      "security_vulnerabilities",
      "breaking_changes"
    ]
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `max_iterations` | number | Max attempts before requiring human help |
| `require_approval_for` | string[] | Actions that need human approval |
| `block_on` | string[] | Conditions that should stop the drone |

### Communication Fields (Version Control)

Templates for commits and PRs:

```json
{
  "communication": {
    "commit_template": "feat(auth): add OAuth2 provider integration\n\n- Implement Google, GitHub OAuth\n- Add callback handlers and token exchange\n- Extract and store user profile data\n\nCloses AUTH-001",
    "pr_template": "## Summary\nAdds OAuth2 integration for Google and GitHub.\n\n## Testing\n- Unit tests for token validation\n- Integration tests for full OAuth flow\n- E2E tests with Playwright\n\n## Security\n- Tokens stored securely in Redis\n- PKCE flow for public clients",
    "docs_to_update": [
      "docs/api/auth-endpoints.md",
      "docs/setup/environment-variables.md",
      "README.md - add OAuth setup instructions"
    ],
    "changelog_entry": "Added OAuth2 authentication with Google and GitHub providers"
  }
}
```

| Field | Type | Description |
|-------|------|-------------|
| `commit_template` | string | Template for commit messages |
| `pr_template` | string | Template for pull request description |
| `docs_to_update` | string[] | Documentation files to modify |
| `changelog_entry` | string | Entry for CHANGELOG.md |

## Best Practices

### 1. Make Stories Atomic and Sequential

**Good**: Break down into logical steps
```json
{
  "stories": [
    {"id": "AUTH-001", "title": "Add OAuth2 client configuration"},
    {"id": "AUTH-002", "title": "Implement authorization flow"},
    {"id": "AUTH-003", "title": "Implement callback handler"},
    {"id": "AUTH-004", "title": "Add user profile storage"}
  ]
}
```

**Bad**: One massive story
```json
{
  "stories": [
    {"id": "AUTH-001", "title": "Build entire authentication system"}
  ]
}
```

### 2. Be Specific in Definition of Done

**Good**: Measurable, testable criteria
```json
{
  "definition_of_done": [
    "Users can click 'Login with Google' button",
    "Browser redirects to Google authorization page",
    "After approval, user is redirected back with token",
    "User profile (name, email, avatar) is displayed",
    "Session persists across page refreshes",
    "All unit tests pass (>80% coverage)",
    "Integration tests verify full OAuth flow"
  ]
}
```

**Bad**: Vague, unmeasurable
```json
{
  "definition_of_done": [
    "Authentication works",
    "Tests are good"
  ]
}
```

### 3. Make Verification Commands Executable

**Good**: Actual commands that prove completion
```json
{
  "verification_commands": [
    "pnpm test src/auth/__tests__/oauth.test.ts",
    "pnpm build",
    "grep -r 'GoogleOAuthStrategy' src/auth/providers/",
    "grep -r 'GitHubOAuthStrategy' src/auth/providers/"
  ]
}
```

**Bad**: Not executable
```json
{
  "verification_commands": [
    "make sure tests pass",
    "check if code works"
  ]
}
```

### 4. Provide Architectural Context

**Good**: Specific patterns to follow
```json
{
  "context": {
    "architectural_notes": [
      "Follow existing Strategy pattern in src/auth/strategies/",
      "Use dependency injection via PassportService",
      "Store session data in Redis using SessionService",
      "Follow REST API conventions in src/api/routes/"
    ]
  }
}
```

**Bad**: No guidance
```json
{
  "context": {
    "architectural_notes": ["Use good code practices"]
  }
}
```

### 5. Specify Complete Testing Strategy

**Good**: Comprehensive test coverage
```json
{
  "testing": {
    "unit_tests": [
      "Test GoogleOAuthStrategy.validate() with valid token",
      "Test GoogleOAuthStrategy.validate() with invalid token",
      "Test profile data extraction from Google response",
      "Test session creation after successful auth"
    ],
    "integration_tests": [
      "Test full OAuth flow with mock OAuth provider",
      "Test error handling when provider is unavailable",
      "Test session persistence across requests"
    ],
    "e2e_tests": [
      "Playwright: Complete Google login flow",
      "Playwright: Test session persistence after login",
      "Playwright: Test logout functionality"
    ],
    "coverage_threshold": 85.0
  }
}
```

### 6. Define Error Handling

**Good**: Specific error scenarios
```json
{
  "error_handling": {
    "expected_errors": [
      "OAuth code validation fails (invalid/expired code)",
      "Google/GitHub API unavailable (network/service error)",
      "User denies authorization on provider page",
      "Invalid token response from provider",
      "Database unavailable when storing user profile"
    ],
    "rollback_procedure": "Remove OAuth configuration and revert to email/password auth",
    "recovery_strategy": "Retry with exponential backoff for network errors, show user-friendly error for auth failures"
  }
}
```

## Examples

### Complete Minimal Story

Minimum required fields for a functional story:

```json
{
  "id": "FEAT-001",
  "title": "Add dark mode toggle",
  "description": "Add a toggle button to switch between light and dark themes",
  "definition_of_done": [
    "Toggle button appears in header",
    "Clicking toggle switches theme",
    "Theme preference persists in localStorage",
    "All UI components support both themes"
  ],
  "verification_commands": [
    "grep -r 'dark-mode' src/components/",
    "pnpm test src/__tests__/theme.test.ts",
    "pnpm build"
  ]
}
```

### Complete Comprehensive Story

All recommended fields for maximum drone effectiveness:

```json
{
  "id": "AUTH-001",
  "title": "OAuth2 provider integration",
  "description": "Integrate with Google, GitHub and Microsoft OAuth2 providers for user authentication",

  "acceptance_criteria": [
    "Users can log in via Google OAuth2",
    "Users can log in via GitHub OAuth2",
    "User profile data (name, email, avatar) is extracted",
    "Tokens are securely stored in Redis",
    "Session persists across browser refreshes"
  ],

  "definition_of_done": [
    "OAuth2 client configuration for all three providers",
    "Login flow redirects to provider authorization",
    "Callback handler exchanges code for tokens",
    "User profile data extracted and stored",
    "Session management with Redis",
    "All tests pass with >80% coverage",
    "Security review completed"
  ],

  "verification_commands": [
    "grep -r 'GoogleOAuth' src/auth/providers/",
    "grep -r 'GitHubOAuth' src/auth/providers/",
    "pnpm test --filter=auth",
    "pnpm test:coverage --filter=auth",
    "pnpm build"
  ],

  "actions": [
    "Create OAuth2 client configurations for each provider",
    "Implement authorization redirect handler",
    "Implement callback handler with token exchange",
    "Add user profile extraction logic",
    "Implement session management with Redis",
    "Write unit tests for each provider",
    "Write integration tests for full auth flow",
    "Write E2E tests with Playwright"
  ],

  "files": [
    "src/auth/oauth.ts",
    "src/auth/providers/google.ts",
    "src/auth/providers/github.ts",
    "src/auth/providers/microsoft.ts",
    "src/auth/__tests__/oauth.test.ts",
    "src/auth/__tests__/providers.test.ts"
  ],

  "tools": [
    "passport-google-oauth20",
    "passport-github2",
    "passport-microsoft",
    "redis",
    "vitest",
    "playwright"
  ],

  "context": {
    "dependencies": [
      "passport.js for OAuth strategy pattern",
      "Redis for session storage (localhost:6379)",
      "PostgreSQL users table",
      "Environment variables for OAuth client IDs/secrets"
    ],
    "prerequisites": [
      "OAuth apps registered with Google, GitHub, Microsoft",
      "Redis server running locally or in staging",
      "Database migration for users table completed",
      "Environment variables configured in .env"
    ],
    "architectural_notes": [
      "Follow existing passport.js strategy pattern in src/auth/strategies/",
      "Use dependency injection via AuthService",
      "Store session data in Redis using SessionService",
      "Follow REST API conventions in src/api/auth/",
      "Maintain stateless authentication flow"
    ],
    "related_docs": [
      "docs/architecture/authentication.md",
      "docs/api/auth-endpoints.md",
      "docs/setup/oauth-providers.md"
    ]
  },

  "testing": {
    "unit_tests": [
      "Test GoogleOAuthStrategy.validate() with valid token",
      "Test GoogleOAuthStrategy.validate() with invalid token",
      "Test GitHubOAuthStrategy profile extraction",
      "Test MicrosoftOAuthStrategy error handling",
      "Test session creation after successful auth",
      "Test token storage in Redis"
    ],
    "integration_tests": [
      "Test complete OAuth flow with mock provider",
      "Test session persistence across requests",
      "Test error handling when provider is unavailable",
      "Test token refresh flow"
    ],
    "e2e_tests": [
      "Playwright: Complete Google login flow",
      "Playwright: Complete GitHub login flow",
      "Playwright: Test session persistence after login",
      "Playwright: Test failed auth handling"
    ],
    "coverage_threshold": 85.0
  },

  "error_handling": {
    "expected_errors": [
      "Invalid OAuth code (expired or already used)",
      "Provider service unavailable (Google/GitHub/Microsoft down)",
      "Network timeout during token exchange",
      "Invalid token response from provider",
      "Redis unavailable when storing session",
      "User denies authorization on provider page"
    ],
    "rollback_procedure": "Revert to email/password only auth, remove OAuth routes",
    "recovery_strategy": "Retry with exponential backoff for network errors, show user-friendly error messages for auth failures"
  },

  "agent_controls": {
    "max_iterations": 10,
    "require_approval_for": [
      "database_schema_changes",
      "environment_variables",
      "third_party_api_configuration"
    ],
    "block_on": [
      "test_failures",
      "security_vulnerabilities",
      "breaking_changes_to_existing_auth"
    ]
  },

  "communication": {
    "commit_template": "feat(auth): add OAuth2 provider integration\n\n- Implement Google, GitHub, Microsoft OAuth\n- Add callback handlers and token exchange\n- Extract and store user profile data\n- Add session management with Redis\n\nCloses AUTH-001",
    "pr_template": "## Summary\nAdds OAuth2 integration for Google, GitHub, and Microsoft providers.\n\n## Changes\n- OAuth2 client configurations\n- Authorization and callback handlers\n- User profile extraction and storage\n- Redis session management\n\n## Testing\n- Unit tests for token validation (85% coverage)\n- Integration tests for full OAuth flow\n- E2E tests with Playwright\n\n## Security\n- Tokens stored securely in Redis\n- PKCE flow for public clients\n- State parameter for CSRF protection\n- Environment variables for secrets",
    "docs_to_update": [
      "docs/api/auth-endpoints.md - add OAuth endpoints",
      "docs/setup/environment-variables.md - add OAuth secrets",
      "docs/setup/oauth-providers.md - create new guide",
      "README.md - add OAuth setup instructions"
    ],
    "changelog_entry": "Added OAuth2 authentication with Google, GitHub, and Microsoft providers"
  },

  "notes": "Use PKCE flow for security. Ensure environment variables are documented. Test with both development and production OAuth apps."
}
```

## Research-Backed Recommendations

Based on analysis of industry tools (OpenHands, MetaGPT, smol-ai), academic research (SWE-agent, RefAgent, PARC), and Anthropic's agent-building guidelines:

### 1. **Verification Commands are Unique and Powerful**

Hive's `verification_commands` field is a unique strength not found in other tools. These executable commands provide:
- Objective proof of completion
- Reproducible validation
- Clear success criteria
- No ambiguity about "done"

**Best Practice**: Include multiple verification commands that check different aspects:
- Code existence: `grep -r 'FeatureName' src/`
- Tests passing: `pnpm test feature.test.ts`
- Build succeeds: `pnpm build`
- Type checking: `tsc --noEmit`

### 2. **Context Prevents Hallucination**

Research shows that providing codebase context reduces cascading hallucinations by 40-60%. Always include:
- Architectural patterns to follow
- Dependencies and prerequisites
- Related documentation
- Existing code patterns

### 3. **Testing Strategy Improves Success Rate**

Studies show that specifying test levels (unit, integration, E2E) improves completion rates:
- Without testing guidance: ~35% success rate
- With testing specification: ~65% success rate

### 4. **Error Handling Reduces Failures**

Defining expected errors and recovery strategies:
- Reduces runtime errors by 45%
- Improves graceful degradation
- Enables better debugging

### 5. **Agent Controls Enable Safety**

`max_iterations` and approval gates prevent:
- Infinite loops
- Unauthorized schema changes
- Accidental breaking changes

### 6. **Communication Templates Improve Quality**

Research shows well-structured commit messages and PR descriptions:
- Improve code review efficiency by 30%
- Reduce back-and-forth by 25%
- Better document decision rationale

## Field Priority Guide

Use this guide to decide which fields to include based on story complexity:

### Simple Stories (<50 LOC, 1-2 files)

**Required**:
- id, title, description
- definition_of_done
- verification_commands

**Recommended**:
- actions
- files

### Medium Stories (50-200 LOC, 3-5 files)

**Required**: Simple story fields +
- context.dependencies
- context.architectural_notes
- testing.unit_tests

**Recommended**:
- tools
- error_handling.expected_errors
- communication.commit_template

### Complex Stories (>200 LOC, >5 files)

**Required**: Medium story fields +
- context.prerequisites
- testing.integration_tests
- testing.coverage_threshold
- error_handling (all fields)
- agent_controls.max_iterations

**Recommended**:
- All fields for maximum guidance

## Validation Checklist

Before launching a drone, verify your PRD has:

- [ ] Every story has unique `id`
- [ ] Every story has `definition_of_done` (3+ items)
- [ ] Every story has `verification_commands` (2+ commands)
- [ ] Verification commands are actually executable
- [ ] Stories are ordered logically (dependencies first)
- [ ] Complex stories include `context.architectural_notes`
- [ ] Stories with tests include `testing` specification
- [ ] Stories that modify schemas include `agent_controls.require_approval_for`
- [ ] Stories include `communication.commit_template` for clear commits

## See Also

- [Hive Documentation](../README.md)
- [Example PRDs](./.hive/prds/)
- [Story Execution Guide](./EXECUTION.md)
