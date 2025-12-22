.PHONY: build install clean test

# Build binary
build:
	go build -o hive .

# Install to /usr/local/bin
install: build
	sudo cp hive /usr/local/bin/hive
	sudo chmod +x /usr/local/bin/hive
	@echo "✅ hive installed to /usr/local/bin/hive"

# Clean build artifacts
clean:
	rm -f hive
	go clean

# Run tests
test:
	go test ./...

# Development mode
dev:
	go run . $(ARGS)
