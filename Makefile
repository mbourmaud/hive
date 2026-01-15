.PHONY: install uninstall help

# Default target
.DEFAULT_GOAL := help

# Installation directory
INSTALL_DIR := $(HOME)/.local/bin
INSTALL_PATH := $(INSTALL_DIR)/hive

help:
	@echo "Hive - Multi-Ralph Orchestration via Git Worktrees"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  install    Copy hive.sh to ~/.local/bin/hive"
	@echo "  uninstall  Remove hive from ~/.local/bin"
	@echo "  help       Show this help message"
	@echo ""
	@echo "After installation, ensure ~/.local/bin is in your PATH:"
	@echo "  export PATH=\"\$$HOME/.local/bin:\$$PATH\""

install:
	@mkdir -p $(INSTALL_DIR)
	@cp hive.sh $(INSTALL_PATH)
	@chmod +x $(INSTALL_PATH)
	@echo "Installed hive to $(INSTALL_PATH)"
	@echo ""
	@echo "Ensure ~/.local/bin is in your PATH:"
	@echo "  export PATH=\"\$$HOME/.local/bin:\$$PATH\""
	@echo ""
	@echo "Run 'hive --help' to get started."

uninstall:
	@if [ -f $(INSTALL_PATH) ]; then \
		rm -f $(INSTALL_PATH); \
		echo "Removed hive from $(INSTALL_PATH)"; \
	else \
		echo "hive is not installed at $(INSTALL_PATH)"; \
	fi
