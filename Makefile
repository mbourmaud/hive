.PHONY: build install clean test lint embed

# Version information
VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo "dev")
GIT_COMMIT ?= $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_DATE ?= $(shell date -u +"%Y-%m-%dT%H:%M:%SZ")

# Build flags
LDFLAGS := -ldflags "-X github.com/mbourmaud/hive/cmd.Version=$(VERSION) \
	-X github.com/mbourmaud/hive/cmd.GitCommit=$(GIT_COMMIT) \
	-X github.com/mbourmaud/hive/cmd.BuildDate=$(BUILD_DATE)"

# Sync embedded files from root to internal/embed/files/
embed:
	@mkdir -p internal/embed/files
	@cp -f docker-compose.yml internal/embed/files/
	@cp -f entrypoint.sh internal/embed/files/
	@cp -rf docker internal/embed/files/
	@cp -rf scripts internal/embed/files/
	@cp -rf templates internal/embed/files/
	@cp -f .env.example internal/embed/files/
	@echo "Embedded files synced"

# Build binary (syncs embedded files first)
build: embed
	go build $(LDFLAGS) -o hive .

# Install to /usr/local/bin
install: build
	sudo cp hive /usr/local/bin/hive
	sudo chmod +x /usr/local/bin/hive
	@echo "hive installed to /usr/local/bin/hive"

# Clean build artifacts
clean:
	rm -f hive
	go clean

# Run tests
test:
	go test -v ./...

# Run tests with coverage
test-coverage:
	go test -v -coverprofile=coverage.out ./...
	go tool cover -html=coverage.out -o coverage.html
	@echo "Coverage report: coverage.html"

# Run linter
lint:
	@which golangci-lint > /dev/null || (echo "Installing golangci-lint..." && go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest)
	golangci-lint run

# Development mode
dev:
	go run . $(ARGS)
