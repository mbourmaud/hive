# Example Tasks for Go REST API

Copy these examples into Queen for your Go projects.

## 1. Microservice Development

```
Build a user service with these independent components:

1. User repository (PostgreSQL)
   - Implement CRUD operations
   - Use sqlx for queries
   - Add transaction support
   - Write repository tests

2. Authentication service
   - JWT token generation
   - Password hashing (bcrypt)
   - Token refresh logic
   - Add service tests

3. HTTP handlers
   - Register, login, profile endpoints
   - Request validation
   - Error handling
   - Add handler tests

4. Middleware stack
   - JWT authentication
   - Rate limiting
   - Request logging
   - CORS configuration

Create 4 parallel tasks.
```

## 2. gRPC Service

```
Add gRPC endpoints alongside REST API:

1. Protocol buffers definition
   - Define service proto
   - Generate Go code
   - Add validation rules
   - Update documentation

2. gRPC server implementation
   - Implement service methods
   - Add interceptors (auth, logging)
   - Connection pooling
   - Write tests

3. gRPC client library
   - Create client package
   - Connection management
   - Retry logic
   - Add client tests

4. Gateway (gRPC-to-REST)
   - Set up grpc-gateway
   - Configure reverse proxy
   - OpenAPI generation
   - Integration tests

Create 4 parallel tasks.
```

## 3. Database Migration

```
Migrate from raw SQL to GORM:

1. Model definitions
   - Convert SQL schemas to GORM models
   - Add associations
   - Custom field types
   - Add model tests

2. Repository layer
   - Rewrite queries with GORM
   - Add query builders
   - Maintain same interface
   - Update repository tests

3. Migration scripts
   - Create GORM migrations
   - Test on staging database
   - Rollback scripts
   - Migration guide

4. Performance testing
   - Benchmark old vs new queries
   - Optimize N+1 queries
   - Add database indexes
   - Document changes

Create 4 parallel tasks.
```

## 4. Testing Suite

```
Comprehensive testing for production:

1. Unit tests
   - Test all handlers (90% coverage)
   - Test all services (95% coverage)
   - Test repositories with mock DB
   - Add table-driven tests

2. Integration tests
   - Test with real PostgreSQL (testcontainers)
   - Test with real Redis
   - Test API endpoints end-to-end
   - Add CI pipeline

3. Load testing
   - Use vegeta or k6
   - Test 1000 req/s scenarios
   - Identify bottlenecks
   - Generate report

4. Contract testing
   - Define API contracts
   - Use Pact or similar
   - Test against contracts
   - Add to CI

Create 4 parallel tasks.
```

## 5. Observability

```
Add monitoring and tracing:

1. Structured logging
   - Implement zerolog or zap
   - Add context to all logs
   - Log levels configuration
   - Add log aggregation

2. Metrics (Prometheus)
   - Add request counter
   - Add response time histogram
   - Add custom business metrics
   - Create Grafana dashboard

3. Distributed tracing (Jaeger)
   - Add OpenTelemetry SDK
   - Instrument all handlers
   - Propagate trace context
   - Configure Jaeger backend

4. Health checks
   - Liveness endpoint
   - Readiness endpoint (check DB, Redis)
   - Metrics endpoint
   - Add to Kubernetes probes

Create 4 parallel tasks.
```

## 6. Security Hardening

```
Security improvements:

1. Input validation
   - Add validator library
   - Validate all DTOs
   - Sanitize user inputs
   - Add validation tests

2. Rate limiting
   - Per-IP rate limiting
   - Per-user rate limiting
   - Distributed rate limiter (Redis)
   - Add tests

3. Security headers
   - Add helmet-like middleware
   - CORS configuration
   - CSP headers
   - Security tests

4. Secrets management
   - Integrate with Vault
   - Rotate secrets
   - Remove hardcoded values
   - Audit code

Create 4 parallel tasks.
```

## Tips for Go Tasks

### Task Granularity

```bash
# ✅ Good
hive-assign drone-1 \
  "Implement UserRepository with CRUD" \
  "Create, Read, Update, Delete methods with sqlx. Add transaction support. Write table-driven tests." \
  "GO-123"

# ❌ Too vague
hive-assign drone-1 "Add database" "..." "GO-124"
```

### Testing Requirements

Every task should run tests before completion:
```bash
go test ./... -v -race -cover
# Must pass with >80% coverage
```

### Dependencies

If using external packages:
```bash
go get github.com/lib/pq
go mod tidy
# Commit go.mod and go.sum
```

### Code Quality

Before task-done:
```bash
# Format code
go fmt ./...

# Lint
golangci-lint run

# Vet
go vet ./...
```
