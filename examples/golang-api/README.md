# Example: Go REST API

This example shows how to use Hive to develop a Go REST API with parallel task execution.

## Project Structure

```
my-api/
├── cmd/
│   └── server/
│       └── main.go
├── internal/
│   ├── handlers/      # HTTP handlers
│   ├── models/        # Data models
│   ├── repository/    # Database layer
│   └── service/       # Business logic
├── pkg/
│   ├── auth/          # JWT authentication
│   └── middleware/    # HTTP middleware
└── go.mod
```

## Setup

1. **Configure Hive for Go:**

```bash
# .env
WORKSPACE_NAME=my-api
HIVE_DOCKERFILE=docker/Dockerfile.golang
GIT_REPO_URL=https://github.com/user/my-api.git
```

2. **Start Hive with 3 workers:**

```bash
hive init --workspace my-api --workers 3 -y
```

## Example Workflow: Build Product Catalog API

### Step 1: Queen Plans the Architecture

Connect to Queen:
```bash
hive connect queen
```

Tell Queen:
```
Build a product catalog REST API with:
- Products CRUD (PostgreSQL)
- Search endpoint with filters
- Image upload to S3
- Rate limiting and auth
```

Queen creates tasks:
```bash
hive-assign drone-1 "Create product handlers" "Implement GET/POST/PUT/DELETE /products endpoints with proper error handling" "API-101"
hive-assign drone-2 "Add product search" "Implement GET /products/search with filters (category, price range, availability)" "API-102"
hive-assign drone-3 "Add image upload" "Implement POST /products/:id/image with S3 upload and resize" "API-103"
```

### Step 2: Workers Execute Tasks

**Terminal 2 - Drone 1 (CRUD Handlers):**
```bash
hive connect 1
take-task
```

Drone 1 implements:
```go
// internal/handlers/products.go
package handlers

import (
    "encoding/json"
    "net/http"
    "github.com/gorilla/mux"
)

type ProductHandler struct {
    service *service.ProductService
}

func (h *ProductHandler) Create(w http.ResponseWriter, r *http.Request) {
    var req CreateProductRequest
    if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
        http.Error(w, err.Error(), http.StatusBadRequest)
        return
    }

    product, err := h.service.Create(r.Context(), req)
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }

    w.Header().Set("Content-Type", "application/json")
    w.WriteHeader(http.StatusCreated)
    json.NewEncoder(w).Encode(product)
}

func (h *ProductHandler) GetByID(w http.ResponseWriter, r *http.Request) {
    vars := mux.Vars(r)
    id := vars["id"]

    product, err := h.service.GetByID(r.Context(), id)
    if err != nil {
        http.Error(w, err.Error(), http.StatusNotFound)
        return
    }

    w.Header().Set("Content-Type", "application/json")
    json.NewEncoder(w).Encode(product)
}

// ... Update, Delete, List handlers
```

Tests:
```bash
go test ./internal/handlers -v -race
```

When tests pass:
```bash
task-done
```

**Terminal 3 - Drone 2 (Search):**
```bash
hive connect 2
take-task
```

Drone 2 implements:
```go
// internal/handlers/search.go
package handlers

import (
    "net/http"
    "strconv"
)

func (h *ProductHandler) Search(w http.ResponseWriter, r *http.Request) {
    query := r.URL.Query()

    filters := service.SearchFilters{
        Category:    query.Get("category"),
        MinPrice:    parseFloat(query.Get("min_price")),
        MaxPrice:    parseFloat(query.Get("max_price")),
        Available:   parseBool(query.Get("available")),
        Page:        parseInt(query.Get("page"), 1),
        PageSize:    parseInt(query.Get("page_size"), 20),
    }

    results, total, err := h.service.Search(r.Context(), filters)
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }

    response := SearchResponse{
        Products: results,
        Total:    total,
        Page:     filters.Page,
        PageSize: filters.PageSize,
    }

    w.Header().Set("Content-Type", "application/json")
    json.NewEncoder(w).Encode(response)
}
```

Service layer:
```go
// internal/service/search.go
func (s *ProductService) Search(ctx context.Context, filters SearchFilters) ([]*models.Product, int, error) {
    query := s.repo.Query()

    if filters.Category != "" {
        query = query.Where("category = ?", filters.Category)
    }
    if filters.MinPrice > 0 {
        query = query.Where("price >= ?", filters.MinPrice)
    }
    if filters.MaxPrice > 0 {
        query = query.Where("price <= ?", filters.MaxPrice)
    }
    if filters.Available {
        query = query.Where("stock > 0")
    }

    total := query.Count()
    products := query.Offset((filters.Page - 1) * filters.PageSize).
                      Limit(filters.PageSize).
                      Find()

    return products, total, nil
}
```

Tests:
```bash
go test ./internal/service -v -race
# Test different filter combinations
# Test pagination
# Test empty results
```

When done:
```bash
task-done
```

**Terminal 4 - Drone 3 (Image Upload):**
```bash
hive connect 3
take-task
```

Drone 3 implements:
```go
// internal/handlers/upload.go
package handlers

import (
    "bytes"
    "context"
    "image"
    "image/jpeg"
    _ "image/png"

    "github.com/aws/aws-sdk-go-v2/service/s3"
    "github.com/nfnt/resize"
)

func (h *ProductHandler) UploadImage(w http.ResponseWriter, r *http.Request) {
    vars := mux.Vars(r)
    productID := vars["id"]

    // Parse multipart form
    if err := r.ParseMultipartForm(10 << 20); err != nil { // 10MB max
        http.Error(w, "File too large", http.StatusBadRequest)
        return
    }

    file, header, err := r.FormFile("image")
    if err != nil {
        http.Error(w, "No file uploaded", http.StatusBadRequest)
        return
    }
    defer file.Close()

    // Decode and resize image
    img, _, err := image.Decode(file)
    if err != nil {
        http.Error(w, "Invalid image", http.StatusBadRequest)
        return
    }

    // Resize to 800x800
    resized := resize.Thumbnail(800, 800, img, resize.Lanczos3)

    // Encode to JPEG
    var buf bytes.Buffer
    if err := jpeg.Encode(&buf, resized, &jpeg.Options{Quality: 85}); err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }

    // Upload to S3
    key := fmt.Sprintf("products/%s/%s", productID, header.Filename)
    _, err = h.s3Client.PutObject(context.Background(), &s3.PutObjectInput{
        Bucket:      aws.String("my-bucket"),
        Key:         aws.String(key),
        Body:        bytes.NewReader(buf.Bytes()),
        ContentType: aws.String("image/jpeg"),
    })
    if err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }

    // Update product with image URL
    imageURL := fmt.Sprintf("https://cdn.example.com/%s", key)
    if err := h.service.UpdateImageURL(context.Background(), productID, imageURL); err != nil {
        http.Error(w, err.Error(), http.StatusInternalServerError)
        return
    }

    w.WriteHeader(http.StatusOK)
    json.NewEncoder(w).Encode(map[string]string{
        "url": imageURL,
    })
}
```

Tests:
```bash
go test ./internal/handlers -v -run TestUploadImage
# Test with valid image
# Test with invalid file
# Test with file too large
# Mock S3 client
```

When done:
```bash
task-done
```

### Step 3: Integration

After all tasks are complete, Queen can assign integration work:
```bash
hive-assign drone-1 "Wire up routes" "Add all handlers to router and test full API" "API-104"
```

```go
// cmd/server/main.go
func main() {
    r := mux.NewRouter()

    // Product routes
    r.HandleFunc("/products", productHandler.List).Methods("GET")
    r.HandleFunc("/products", productHandler.Create).Methods("POST")
    r.HandleFunc("/products/{id}", productHandler.GetByID).Methods("GET")
    r.HandleFunc("/products/{id}", productHandler.Update).Methods("PUT")
    r.HandleFunc("/products/{id}", productHandler.Delete).Methods("DELETE")
    r.HandleFunc("/products/search", productHandler.Search).Methods("GET")
    r.HandleFunc("/products/{id}/image", productHandler.UploadImage).Methods("POST")

    log.Fatal(http.ListenAndServe(":8080", r))
}
```

## Common Tasks

### Add Middleware

```bash
hive-assign drone-1 "Add JWT auth middleware" "Protect all routes except /login with JWT validation" "API-105"
hive-assign drone-2 "Add rate limiting" "Implement rate limiter: 100 req/min per IP" "API-106"
hive-assign drone-3 "Add request logging" "Log all requests with duration and status" "API-107"
```

### Performance Optimization

```bash
hive-assign drone-1 "Add database indexes" "Create indexes on products(category), products(price)" "API-108"
hive-assign drone-2 "Add Redis caching" "Cache GET /products/:id for 5min" "API-109"
hive-assign drone-3 "Add connection pooling" "Configure database connection pool (max 25)" "API-110"
```

### Testing

```bash
hive-assign drone-1 "Add integration tests" "Test full CRUD flow with real DB" "API-111"
hive-assign drone-2 "Add benchmark tests" "Benchmark search endpoint with 10k products" "API-112"
hive-assign drone-3 "Add E2E tests" "Test complete user journey with Playwright" "API-113"
```

## Best Practices

### 1. Use Context Everywhere

```go
// ✅ Good
func (s *Service) GetUser(ctx context.Context, id string) (*User, error) {
    return s.repo.FindByID(ctx, id)
}

// ❌ Bad
func (s *Service) GetUser(id string) (*User, error) {
    return s.repo.FindByID(id)
}
```

### 2. Proper Error Handling

```go
// ✅ Good
if err != nil {
    return fmt.Errorf("failed to create product: %w", err)
}

// ❌ Bad
if err != nil {
    return err  // Loses context
}
```

### 3. Test Coverage

```bash
# Before task-done
go test ./... -cover -v
# Require >80% coverage
```

## Troubleshooting

### Go Module Issues

```bash
# Update dependencies
go mod tidy
go mod download

# Verify go.sum
go mod verify
```

### Database Connection

```bash
# Test connection
go run cmd/server/main.go
# Check logs for connection errors

# Run migrations
migrate -path ./migrations -database "postgres://..." up
```

### Hot Reload Not Working

```bash
# Check air is running
ps aux | grep air

# Restart air
air -c .air.toml
```

## Example Timeline

**Sequential (1 developer):**
- CRUD handlers: 3 hours
- Search: 2 hours
- Image upload: 2 hours
- Integration: 1 hour
- **Total: 8 hours**

**Parallel (Hive with 3 workers):**
- All tasks in parallel: 3 hours
- Integration: 1 hour
- **Total: 4 hours**

**Time saved: 50%**
