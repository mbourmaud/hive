.PHONY: build install clean test lint web-install web-build web-dev

VERSION ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo "dev")
GIT_COMMIT ?= $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_DATE ?= $(shell date -u +"%Y-%m-%dT%H:%M:%SZ")

LDFLAGS := -ldflags "-X github.com/mbourmaud/hive/cmd.Version=$(VERSION) \
	-X github.com/mbourmaud/hive/cmd.GitCommit=$(GIT_COMMIT) \
	-X github.com/mbourmaud/hive/cmd.BuildDate=$(BUILD_DATE)"

build: web-build
	go build $(LDFLAGS) -o hive .

install: build
	@cat hive | sudo tee /usr/local/bin/hive > /dev/null
	sudo chmod +x /usr/local/bin/hive
	@echo "hive installed to /usr/local/bin/hive"

clean:
	rm -f hive
	rm -rf internal/monitor/dist
	go clean

test:
	go test -v ./...

test-coverage:
	go test -v -coverprofile=coverage.out ./...
	go tool cover -html=coverage.out -o coverage.html
	@echo "Coverage report: coverage.html"

test-all: test
	@echo "All tests passed!"

lint:
	@which golangci-lint > /dev/null || (echo "Installing golangci-lint..." && go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest)
	golangci-lint run

dev:
	go run . $(ARGS)

web-install:
	cd web && npm install

web-build: web-install
	cd web && npm run build

web-dev:
	cd web && npm run dev
