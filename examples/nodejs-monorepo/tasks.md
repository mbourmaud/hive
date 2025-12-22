# Example Tasks for Node.js Monorepo

Copy these examples into Queen to practice using Hive.

## 1. Bug Fixing Sprint

```
Fix these 4 bugs in parallel:

1. Bug: Login timeout after 5 minutes
   - Location: packages/api/src/auth
   - Issue: SESSION-123
   - Fix: Increase session timeout to 30 minutes

2. Bug: CSV export returns empty file
   - Location: packages/api/src/exports
   - Issue: SESSION-124
   - Fix: Handle empty data case in ExportService

3. Bug: Email validation rejects valid addresses
   - Location: packages/web/src/utils/validation.ts
   - Issue: SESSION-125
   - Fix: Update regex to support plus addressing

4. Bug: Date picker shows wrong timezone
   - Location: packages/web/src/components/DatePicker
   - Issue: SESSION-126
   - Fix: Convert all dates to UTC before display

Create 4 parallel tasks, one per bug.
```

## 2. Feature: Add Dashboard

```
Implement analytics dashboard with these independent components:

1. Backend: User statistics API
   - GET /analytics/users (total, active, new today)
   - Use Prisma aggregations
   - Add caching with Redis
   - Write integration tests

2. Backend: Revenue statistics API
   - GET /analytics/revenue (total, today, this month)
   - Query orders table
   - Add caching
   - Write tests

3. Frontend: Statistics cards component
   - Display user/revenue metrics
   - Real-time updates every 30s
   - Loading states
   - Error handling

4. Frontend: Charts component
   - Line chart for user growth
   - Bar chart for revenue
   - Use recharts library
   - Responsive design

Create 4 parallel tasks for implementation.
```

## 3. Refactoring: Migrate to Prisma

```
Migrate from TypeORM to Prisma across all packages:

1. Database schema migration
   - Convert TypeORM entities to Prisma schema
   - Generate migrations
   - Test on dev database
   - Document breaking changes

2. API layer refactoring
   - Update all repositories to use Prisma Client
   - Update query builders
   - Maintain same API contracts
   - Update unit tests

3. Update integration tests
   - Rewrite test fixtures for Prisma
   - Update test database setup
   - Verify all tests pass
   - Update CI configuration

4. Documentation update
   - Update README with Prisma setup
   - Update API docs
   - Create migration guide
   - Update developer onboarding

Create 4 parallel tasks for the migration.
```

## 4. Testing: Add E2E Coverage

```
Add end-to-end tests for critical user flows:

1. Authentication flow
   - Test: Register → Login → Refresh token → Logout
   - Use Playwright
   - Test both happy path and error cases
   - Add to CI pipeline

2. User CRUD flow
   - Test: Create user → Read → Update → Delete
   - Verify UI updates
   - Test form validations
   - Test error handling

3. Payment flow (if applicable)
   - Test: Add to cart → Checkout → Payment → Confirmation
   - Mock payment gateway
   - Test success and failure cases
   - Verify email notifications

4. Search and filtering
   - Test: Search users → Apply filters → Pagination
   - Test edge cases (empty results)
   - Test performance (large datasets)
   - Verify accessibility

Create 4 parallel tasks for E2E testing.
```

## 5. Performance Optimization

```
Optimize application performance across layers:

1. Database optimization
   - Add missing indexes (analyze slow query log)
   - Optimize N+1 queries with eager loading
   - Add database connection pooling
   - Benchmark improvements

2. API response time
   - Add response caching with Redis
   - Implement pagination for list endpoints
   - Add compression middleware
   - Profile and fix slow endpoints

3. Frontend bundle size
   - Code split by route
   - Lazy load heavy components
   - Optimize images (WebP, lazy loading)
   - Analyze and reduce bundle size

4. Frontend render performance
   - Memoize expensive components
   - Virtualize long lists
   - Debounce search inputs
   - Add React DevTools profiling

Create 4 parallel tasks for optimization work.
```

## 6. Security Audit

```
Security improvements across the application:

1. API security hardening
   - Add rate limiting (express-rate-limit)
   - Implement CSRF protection
   - Add helmet.js security headers
   - Audit dependencies (npm audit)

2. Authentication improvements
   - Add 2FA support
   - Implement account lockout after failed attempts
   - Add password strength requirements
   - Add security event logging

3. Data validation
   - Add input validation (class-validator)
   - Sanitize user inputs (XSS prevention)
   - Add SQL injection tests
   - Validate file uploads

4. Secrets management
   - Move secrets to environment variables
   - Add secrets rotation
   - Implement vault integration
   - Audit for hardcoded secrets

Create 4 parallel tasks for security improvements.
```

## Tips for Writing Good Tasks

### Task Structure

```bash
hive-assign <drone> "<title>" "<description>" "<ticket-id>"

# Good example:
hive-assign drone-1 \
  "Add user pagination API" \
  "Add GET /users?page=1&limit=10 with cursor-based pagination. Include total count. Write integration tests." \
  "PROJ-456"
```

### Task Granularity

**Too broad:**
```bash
hive-assign drone-1 "Implement user management" "..." "PROJ-123"
# ❌ Too vague, unclear scope
```

**Too narrow:**
```bash
hive-assign drone-1 "Add import statement" "Import UserService at line 1" "PROJ-124"
# ❌ Too trivial, not worth a task
```

**Just right:**
```bash
hive-assign drone-1 "Create UserService with CRUD methods" "Implement findAll, findById, create, update, delete. Add unit tests for each method." "PROJ-125"
# ✅ Clear scope, testable, complete unit of work
```

### Dependencies

**Bad (tasks depend on each other):**
```bash
hive-assign drone-1 "Create User API"
hive-assign drone-2 "Create UI that calls User API"  # Blocked!
```

**Good (independent tasks):**
```bash
hive-assign drone-1 "Create User API"
hive-assign drone-2 "Create UI components (mock API for now)"
# Later: drone-2 updates to use real API when drone-1 is done
```

### Verification

Every task should specify how to verify it's complete:

```bash
hive-assign drone-1 \
  "Fix login timeout bug" \
  "Increase session timeout to 30min in auth.config.ts. Verify: 1) pnpm test auth passes, 2) Manual test: login and wait 15min, should stay logged in" \
  "BUG-789"
```
