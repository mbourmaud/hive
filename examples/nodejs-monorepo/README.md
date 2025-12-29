# Example: Node.js Monorepo

This example shows how to use Hive to develop a full-stack TypeScript monorepo with parallel task execution.

## Project Structure

```
my-app/
├── packages/
│   ├── api/           # NestJS REST API
│   ├── web/           # React frontend
│   ├── shared/        # Shared types
│   └── workers/       # Background jobs
├── package.json       # Root package.json
└── pnpm-workspace.yaml
```

## Setup

1. **Configure Hive for Node.js:**

```bash
# .env
WORKSPACE_NAME=my-app
HIVE_DOCKERFILE=docker/Dockerfile.node
GIT_REPO_URL=https://github.com/user/my-app.git
```

2. **Start Hive with 4 workers:**

```bash
hive init --workspace my-app --workers 4 -y
```

## Example Workflow: Add User Management Feature

### Step 1: Queen Breaks Down the Feature

Connect to Queen:
```bash
hive connect queen
```

Tell Queen:
```
I need to add user management with CRUD operations, authentication, and a UI.
Please create parallel tasks for the team.
```

Queen creates tasks:
```bash
hive-assign drone-1 "Create user database schema" "Add Prisma schema for User model with migrations" "PROJ-123"
hive-assign drone-2 "Build user REST API" "Create NestJS CRUD endpoints for users" "PROJ-124"
hive-assign drone-3 "Create UI components" "Build React forms for user management" "PROJ-125"
hive-assign drone-4 "Add authentication" "Implement JWT auth with refresh tokens" "PROJ-126"
```

### Step 2: Workers Execute Tasks

**Terminal 2 - Drone 1 (Database):**
```bash
hive connect 1
# Auto-runs: my-tasks
# Shows: "Task: Create user database schema"
take-task
```

Drone 1 creates:
```prisma
// packages/api/prisma/schema.prisma
model User {
  id        String   @id @default(uuid())
  email     String   @unique
  name      String
  password  String
  role      Role     @default(USER)
  createdAt DateTime @default(now())
}

enum Role {
  USER
  ADMIN
}
```

Runs migration:
```bash
cd packages/api
pnpm prisma migrate dev --name add_users
```

Tests:
```bash
pnpm test:db
```

When done:
```bash
task-done
```

**Terminal 3 - Drone 2 (API):**
```bash
hive connect 2
take-task
```

Drone 2 creates:
```typescript
// packages/api/src/users/users.controller.ts
@Controller('users')
export class UsersController {
  @Get()
  findAll(@Query() query: PaginationDto) {
    return this.usersService.findAll(query);
  }

  @Post()
  create(@Body() dto: CreateUserDto) {
    return this.usersService.create(dto);
  }

  @Put(':id')
  update(@Param('id') id: string, @Body() dto: UpdateUserDto) {
    return this.usersService.update(id, dto);
  }

  @Delete(':id')
  remove(@Param('id') id: string) {
    return this.usersService.remove(id);
  }
}
```

Tests:
```bash
cd packages/api
pnpm test users.controller.spec.ts
```

When CI passes:
```bash
task-done
```

**Terminal 4 - Drone 3 (UI):**
```bash
hive connect 3
take-task
```

Drone 3 creates:
```tsx
// packages/web/src/components/UserForm.tsx
export function UserForm({ userId }: { userId?: string }) {
  const form = useForm<UserFormData>({
    defaultValues: userId ? useUser(userId) : {},
  });

  return (
    <Form {...form}>
      <FormField name="email" label="Email" type="email" />
      <FormField name="name" label="Name" />
      <FormField name="role" label="Role" type="select" />
      <Button type="submit">Save</Button>
    </Form>
  );
}
```

Tests:
```bash
cd packages/web
pnpm test UserForm.test.tsx
pnpm build  # Verify no type errors
```

When done:
```bash
task-done
```

**Terminal 5 - Drone 4 (Auth):**
```bash
hive connect 4
take-task
```

Drone 4 implements:
```typescript
// packages/api/src/auth/auth.service.ts
@Injectable()
export class AuthService {
  async login(dto: LoginDto) {
    const user = await this.validateUser(dto.email, dto.password);
    const tokens = await this.generateTokens(user);
    return { user, ...tokens };
  }

  async refreshToken(refreshToken: string) {
    const payload = await this.verifyToken(refreshToken);
    return this.generateTokens(payload.sub);
  }
}
```

Tests:
```bash
cd packages/api
pnpm test auth.service.spec.ts
```

When done:
```bash
task-done
```

### Step 3: Queen Monitors Progress

Queen checks status:
```bash
hive-status
```

Output:
```
📊 HIVE Status
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Active Workers: 4/4
Queue Size: 0
Failed Tasks: 0

✅ drone-1: Create user database schema [DONE]
✅ drone-2: Build user REST API [DONE]
✅ drone-3: Create UI components [DONE]
✅ drone-4: Add authentication [DONE]
```

## Common Tasks

### Run Integration Tests

Queen assigns:
```bash
hive-assign drone-1 "Run integration tests" "Test full user flow end-to-end" "PROJ-127"
```

Drone 1:
```bash
take-task
cd packages/api
pnpm test:integration users.e2e.test.ts
task-done
```

### Fix TypeScript Errors

If `pnpm build` fails with type errors:
```bash
hive-assign drone-1 "Fix type error in UserForm" "Add missing props type" "PROJ-128"
hive-assign drone-2 "Fix type error in API" "Update UserDto interface" "PROJ-129"
```

### Update Documentation

```bash
hive-assign drone-3 "Document user API" "Add OpenAPI specs for /users endpoints" "PROJ-130"
```

## Best Practices

### 1. Independent Tasks

✅ **Good** (parallel):
```bash
hive-assign drone-1 "Add user schema"
hive-assign drone-2 "Add product schema"
hive-assign drone-3 "Add order schema"
```

❌ **Bad** (sequential dependency):
```bash
hive-assign drone-1 "Create user API"
hive-assign drone-2 "Create UI that calls user API"  # Blocked!
```

### 2. Atomic Commits

Each worker should:
```bash
git add packages/api/src/users
git commit -m "feat(api): add user CRUD endpoints

- Create UsersController with CRUD operations
- Add UsersService with business logic
- Add integration tests
- Update OpenAPI schema

Closes PROJ-124"
```

### 3. CI Before task-done

**Never** mark a task as done if CI fails:
```bash
# ❌ Bad
pnpm test  # FAILED
task-done  # DON'T DO THIS

# ✅ Good
pnpm test  # FAILED
task-failed "Integration test failing: user creation returns 500"
```

## Troubleshooting

### TypeScript Build Fails

```bash
# Inside worker container
cd packages/api
pnpm build --verbose

# Check for missing dependencies
pnpm install
```

### Database Connection Issues

```bash
# Check Prisma connection
cd packages/api
pnpm prisma studio

# Reset database (dev only!)
pnpm prisma migrate reset
```

### Port Conflicts

If pnpm dev fails with "port already in use":
```bash
# Each worker uses different ports
# Drone 1: API on 3001, web on 5001
# Drone 2: API on 3002, web on 5002
# etc.

# Set in .env.local per worker
PORT=300$DRONE_ID
WEB_PORT=500$DRONE_ID
```

## Example Tasks for Queen

Copy-paste these into Queen to get started:

```
Break down this feature into parallel tasks:

Feature: User Management System
- Database schema (User model with email, name, role)
- REST API (CRUD endpoints for users)
- React UI (user list, create form, edit form)
- Authentication (JWT with refresh tokens)
- Tests (unit + integration for all layers)
- Documentation (API docs + README)

We have 4 workers available. Please create tasks that can be done in parallel.
```

## Expected Timeline

With 4 workers working in parallel:
- **Sequential (1 person):** ~8 hours
- **Parallel (Hive with 4 workers):** ~2 hours

Time saved: **75%**
