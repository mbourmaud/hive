.PHONY: install uninstall release test lint help

VERSION := $(shell grep '^VERSION=' hive.sh | cut -d'"' -f2)
INSTALL_DIR := $(HOME)/.local/bin
COMMANDS_DIR := $(HOME)/.claude/commands

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

install: ## Install hive CLI and skills locally
	@mkdir -p $(INSTALL_DIR)
	@cp hive.sh $(INSTALL_DIR)/hive
	@chmod +x $(INSTALL_DIR)/hive
	@echo "✓ CLI installed to $(INSTALL_DIR)/hive"
	@mkdir -p $(COMMANDS_DIR)
	@cp commands/*.md $(COMMANDS_DIR)/
	@echo "✓ Skills installed to $(COMMANDS_DIR)/"
	@echo ""
	@echo "Done! Run 'hive --version' to verify."

uninstall: ## Remove hive CLI and skills
	@rm -f $(INSTALL_DIR)/hive
	@rm -f $(COMMANDS_DIR)/hive:*.md
	@echo "✓ Hive uninstalled"

test: ## Run tests
	@echo "Testing hive.sh..."
	@bash -n hive.sh
	@./hive.sh --version
	@echo "Testing install.sh..."
	@bash -n install.sh
	@echo "Checking skills..."
	@ls commands/*.md | wc -l | xargs -I {} echo "✓ {} skills found"
	@echo ""
	@echo "All tests passed!"

lint: ## Lint shell scripts
	@shellcheck hive.sh install.sh || true

release: ## Create a new release (usage: make release V=0.3.0)
ifndef V
	$(error VERSION is not set. Usage: make release V=0.3.0)
endif
	@echo "Creating release v$(V)..."
	@sed -i '' 's/^VERSION=".*"/VERSION="$(V)"/' hive.sh
	@sed -i '' 's/^VERSION=".*"/VERSION="$(V)"/' install.sh
	@git add hive.sh install.sh
	@git commit -m "chore: Bump version to $(V)"
	@git tag -a "v$(V)" -m "Release v$(V)"
	@echo ""
	@echo "Release v$(V) created!"
	@echo "Run 'git push && git push --tags' to publish."

version: ## Show current version
	@echo "v$(VERSION)"
