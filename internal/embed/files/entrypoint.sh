#!/bin/bash

# ===========================================
# Claude Agent Entrypoint
# Configures authentication for all CLI tools
# ===========================================

# Timestamp helper (format configurable via LOG_TIMESTAMP_FORMAT env var)
log() {
    local fmt="${LOG_TIMESTAMP_FORMAT:-%x %H:%M}"
    echo "[$(date +"$fmt")] $*"
}

log "Initializing Claude Agent ${AGENT_ID:-unknown}..."

# ============================================
# Add Hive scripts to PATH
# ============================================

# Make hive scripts executable and add to PATH
if [ -d "/hive-config/scripts/bin" ]; then
    chmod +x /hive-config/scripts/bin/* 2>/dev/null || true
    export PATH="/hive-config/scripts/bin:$PATH"
    log "[+] Added hive scripts to PATH"
fi

if [ -d "/hive-config/scripts/redis" ]; then
    chmod +x /hive-config/scripts/redis/* 2>/dev/null || true
fi

# ============================================
# Claude Configuration Setup
# ============================================

# ~/.claude is now PARTIALLY SHARED (MCPs, plugins, projects from host)
# settings.json, history.jsonl, and session-env are ISOLATED per agent

log "[+] Claude configuration: PARTIAL (MCPs, plugins, projects shared)"
log "[+] Conversation history: ISOLATED (this agent only)"

# Ensure ~/.claude directory and subdirectories exist
mkdir -p ~/.claude/projects ~/.claude/mcps ~/.claude/plugins

# Ensure pnpm directories exist and are writable
mkdir -p ~/.local/share/pnpm/{store,.tools,state}
chmod -R u+w ~/.local/share/pnpm 2>/dev/null || true

# Ensure node_modules cache directory exists and is writable
mkdir -p ~/node_modules_cache
chmod -R u+w ~/node_modules_cache 2>/dev/null || true

# Ensure workspace node_modules volume is writable (Docker volume may be owned by root)
if [ -d "/workspace/node_modules" ]; then
    sudo chown -R agent:agent /workspace/node_modules 2>/dev/null || true
fi

# Create ~/.claude.json with OAuth and onboarding flags to bypass setup wizard
if [ -n "$CLAUDE_CODE_OAUTH_TOKEN" ]; then
    log "[+] Configuring Claude OAuth and bypassing setup wizard..."
    jq -n \
      --arg token "$CLAUDE_CODE_OAUTH_TOKEN" \
      '{
        hasCompletedOnboarding: true,
        bypassPermissionsModeAccepted: true,
        lastOnboardingVersion: "2.0.76",
        oauthAccount: {
          accessToken: $token
        }
      }' > ~/.claude.json

    # Validate JSON
    if ! jq empty ~/.claude.json 2>/dev/null; then
        log "[!] ERROR: Generated invalid JSON in ~/.claude.json"
        cat ~/.claude.json
        exit 1
    fi

    chmod 600 ~/.claude.json
    log "[+] Created ~/.claude.json with OAuth token and onboarding bypass"
fi

# Configure MCPs in ~/.claude.json (where Claude Code reads them)
# Claude reads MCPs from projects["/workspace"].mcpServers, not from settings.json

if command -v python3 &> /dev/null; then
    log "[+] Configuring MCPs in ~/.claude.json..."

    python3 << 'PYTHON_SCRIPT'
import json
import os

# Collect MCPs from host + Hive built-in
mcps = {}

# 1. Load MCPs from host settings (copied to .hive/host-mcps.json during init)
host_mcps_path = "/hive-config/host-mcps.json"
if os.path.exists(host_mcps_path):
    try:
        with open(host_mcps_path, "r") as f:
            host_settings = json.load(f)

        host_mcps = host_settings.get("mcpServers", {})
        for name, config in host_mcps.items():
            mcps[name] = config
            print(f"  - {name}: (from host)")
    except Exception as e:
        print(f"Warning: Could not parse host MCPs: {e}")

# 2. Always add the Hive MCP for elegant task management
mcps["hive"] = {
    "command": "node",
    "args": ["/hive-config/scripts/mcp/hive-mcp.js"]
}
print("  - hive: (built-in)")

# 3. Read existing ~/.claude.json (has OAuth token from earlier in entrypoint)
claude_json_path = os.path.expanduser("~/.claude.json")
claude_json = {}
if os.path.exists(claude_json_path):
    try:
        with open(claude_json_path, "r") as f:
            claude_json = json.load(f)
    except:
        pass

# 4. Add MCPs under projects["/workspace"].mcpServers
#    This is where Claude Code reads MCPs from for /mcp command
if "projects" not in claude_json:
    claude_json["projects"] = {}

claude_json["projects"]["/workspace"] = {
    "mcpServers": mcps,
    "allowedTools": []  # Allow all tools
}

# 5. Write back to ~/.claude.json
with open(claude_json_path, "w") as f:
    json.dump(claude_json, f, indent=2)

print(f"Total MCPs: {len(mcps)} (configured in ~/.claude.json)")

# 6. Also create ~/.claude/settings.json for permissions
settings = {
    "permissions": {
        "defaultMode": "bypassPermissions"
    }
}
os.makedirs(os.path.expanduser("~/.claude"), exist_ok=True)
with open(os.path.expanduser("~/.claude/settings.json"), "w") as f:
    json.dump(settings, f, indent=2)
PYTHON_SCRIPT

    if [ $? -eq 0 ]; then
        log "[+] MCPs configured in ~/.claude.json"
    else
        log "[!] Failed to configure MCPs"
    fi
else
    # Fallback: minimal settings
    cat > ~/.claude/settings.json << 'EOF'
{
  "permissions": {
    "defaultMode": "bypassPermissions"
  },
  "mcpServers": {
    "hive": {
      "command": "node",
      "args": ["/hive-config/scripts/mcp/hive-mcp.js"]
    }
  }
}
EOF
    log "[+] Created minimal settings.json (no hive.yaml or python3)"
fi

chmod 600 ~/.claude/settings.json

# Symlink project CLAUDE.md to home directory for Claude to find
# CLAUDE.md is synced to .hive/ during init, so we read from /hive-config/
if [ -f "/hive-config/CLAUDE.md" ]; then
    ln -sf /hive-config/CLAUDE.md ~/CLAUDE.md
    log "[+] Linked project CLAUDE.md to ~/CLAUDE.md"
fi

# Symlink role-specific instructions based on AGENT_ROLE
# Each agent gets their role's template + both templates for reference
if [ "$AGENT_ROLE" = "queen" ] || [ "$AGENT_ROLE" = "orchestrator" ]; then
    if [ -f "/hive-config/templates/CLAUDE-QUEEN.md" ]; then
        ln -sf /hive-config/templates/CLAUDE-QUEEN.md ~/CLAUDE-ROLE.md
        log "[+] Linked Queen instructions to ~/CLAUDE-ROLE.md"
    fi
else
    if [ -f "/hive-config/templates/CLAUDE-WORKER.md" ]; then
        ln -sf /hive-config/templates/CLAUDE-WORKER.md ~/CLAUDE-ROLE.md
        log "[+] Linked Worker instructions to ~/CLAUDE-ROLE.md"
    fi
fi

# Make both role templates available for reference (agents can read each other's instructions)
if [ -f "/hive-config/templates/CLAUDE-QUEEN.md" ]; then
    ln -sf /hive-config/templates/CLAUDE-QUEEN.md ~/CLAUDE-QUEEN.md
fi
if [ -f "/hive-config/templates/CLAUDE-WORKER.md" ]; then
    ln -sf /hive-config/templates/CLAUDE-WORKER.md ~/CLAUDE-WORKER.md
fi

# Create hive-config symlink in workspace for relative path access
# Allows agents to read hive-config/hive.yaml from /workspace
ln -sf /hive-config /workspace/hive-config
log "[+] Linked /hive-config to /workspace/hive-config"

# Ensure isolated conversation files exist
if [ ! -f ~/.claude/history.jsonl ]; then
    touch ~/.claude/history.jsonl
    log "[+] Created isolated history.jsonl"
fi

if [ ! -d ~/.claude/session-env ]; then
    mkdir -p ~/.claude/session-env
    log "[+] Created isolated session-env directory"
fi

# ============================================
# SSH Configuration (fix macOS-specific options for Linux)
# ============================================

# ~/.ssh is mounted read-only from host, so we need to create a local copy
if [ -d ~/.ssh ] && [ -f ~/.ssh/config ]; then
    # Check if config has macOS-specific options that break on Linux
    if grep -q "UseKeychain" ~/.ssh/config 2>/dev/null; then
        log "[+] Fixing SSH config (removing macOS-specific options)..."

        # Create local SSH directory and copy files
        mkdir -p ~/.ssh-local
        cp ~/.ssh/* ~/.ssh-local/ 2>/dev/null || true

        # Fix the config file
        sed -e 's/UseKeychain.*//g' -e 's/AddKeysToAgent.*//g' ~/.ssh-local/config > ~/.ssh-local/config.tmp
        mv ~/.ssh-local/config.tmp ~/.ssh-local/config
        chmod 600 ~/.ssh-local/config
        chmod 600 ~/.ssh-local/id_* 2>/dev/null || true

        # Set GIT_SSH_COMMAND to use our fixed config
        export GIT_SSH_COMMAND="ssh -F ~/.ssh-local/config"
        log "[+] SSH config fixed, using ~/.ssh-local/config"
    fi
fi

# ============================================
# Git Configuration
# ============================================

log "[+] Configuring Git..."

# Only configure if .gitconfig is writable (not mounted read-only from host)
if [ -w ~/.gitconfig ] || [ ! -f ~/.gitconfig ]; then
    if [ -n "$GIT_USER_EMAIL" ]; then
        git config --global user.email "$GIT_USER_EMAIL" 2>/dev/null || true
    fi

    if [ -n "$GIT_USER_NAME" ]; then
        git config --global user.name "$GIT_USER_NAME" 2>/dev/null || true
    fi
else
    log "[+] Git config mounted from host (read-only), skipping git config"
fi

# ============================================
# GitHub CLI (gh)
# ============================================

if [ -n "$GITHUB_TOKEN" ] && command -v gh &> /dev/null; then
    log "[+] Configuring GitHub CLI..."
    mkdir -p ~/.config/gh
    cat > ~/.config/gh/hosts.yml << EOF
github.com:
    oauth_token: ${GITHUB_TOKEN}
    user: $(echo "$GIT_USER_EMAIL" | cut -d@ -f1)
    git_protocol: ssh
EOF
    chmod 600 ~/.config/gh/hosts.yml
fi

# ============================================
# GitLab CLI (glab)
# ============================================

if [ -n "$GITLAB_TOKEN" ] && command -v glab &> /dev/null; then
    log "[+] Configuring GitLab CLI..."
    mkdir -p ~/.config/glab-cli
    cat > ~/.config/glab-cli/config.yml << EOF
hosts:
  ${GITLAB_HOST:-gitlab.com}:
    token: ${GITLAB_TOKEN}
    api_protocol: https
    git_protocol: ssh
EOF
    chmod 600 ~/.config/glab-cli/config.yml
fi

# ============================================
# Workspace Initialization
# ============================================

WORKSPACE_DIR="/workspace/${WORKSPACE_NAME:-my-project}"

# Check if /workspace is already a git worktree (created by hive init)
if [ -d "/workspace/.git" ] || [ -f "/workspace/.git" ]; then
    log "[+] Workspace already initialized as git worktree"
    WORKSPACE_DIR="/workspace"

    # Fix worktree .git file to point to mounted git directory
    if [ -f "/workspace/.git" ] && [ -d "/workspace-git" ]; then
        # Read the gitdir path from .git file
        GITDIR_PATH=$(cat /workspace/.git | sed 's/gitdir: //')
        # Extract worktree name from path (e.g., /path/.git/worktrees/queen -> queen)
        WORKTREE_NAME=$(basename "$GITDIR_PATH")
        # Update .git file to point to container's mounted git directory
        echo "gitdir: /workspace-git/worktrees/$WORKTREE_NAME" > /workspace/.git
        log "[+] Fixed git worktree path to use mounted repository"
    fi
else
    # Fallback: Initialize workspace if it doesn't exist
    if [ ! -d "$WORKSPACE_DIR" ]; then
        log "[+] Initializing workspace: $WORKSPACE_DIR"
        mkdir -p "$WORKSPACE_DIR"

        # If git repo URL is provided, clone it
        if [ -n "$GIT_REPO_URL" ]; then
            log "[+] Cloning repository: $GIT_REPO_URL"
            git clone "$GIT_REPO_URL" "$WORKSPACE_DIR"
        else
            # Initialize empty git repo
            cd "$WORKSPACE_DIR"
            git init
            echo "# ${WORKSPACE_NAME:-my-project}" > README.md
            git add README.md
            git commit -m "Initial commit" || true
        fi
    fi
fi

# ============================================
# CLI Tools Installation (from hive.yaml)
# ============================================

TOOLS_CACHE="/home/agent/.tools-cache"
TOOLS_INSTALLED="$TOOLS_CACHE/installed.txt"
mkdir -p "$TOOLS_CACHE"
mkdir -p ~/.local/bin
export PATH="$HOME/.local/bin:$PATH"

install_tool() {
    local tool=$1
    local ARCH=$(dpkg --print-architecture)

    # Skip if already installed and cached
    if grep -q "^$tool$" "$TOOLS_INSTALLED" 2>/dev/null; then
        log "[+] Tool '$tool' already installed (cached)"
        return 0
    fi

    case "$tool" in
        glab)
            log "[+] Installing GitLab CLI..."
            if [ "$ARCH" = "arm64" ]; then
                URL="https://gitlab.com/gitlab-org/cli/-/releases/v1.50.0/downloads/glab_1.50.0_linux_arm64.tar.gz"
            else
                URL="https://gitlab.com/gitlab-org/cli/-/releases/v1.50.0/downloads/glab_1.50.0_linux_amd64.tar.gz"
            fi
            wget -q "$URL" -O /tmp/glab.tar.gz
            tar -xzf /tmp/glab.tar.gz -C /tmp
            sudo install -o root -g root -m 0755 /tmp/bin/glab /usr/local/bin/glab
            rm -rf /tmp/glab* /tmp/bin
            ;;
        psql)
            log "[+] Installing PostgreSQL client..."
            sudo apt-get update -qq && sudo apt-get install -y -qq postgresql-client
            ;;
        mongosh)
            log "[+] Installing MongoDB shell..."
            if [ "$ARCH" = "arm64" ]; then
                URL="https://downloads.mongodb.com/compass/mongodb-mongosh_2.1.1_arm64.deb"
            else
                URL="https://downloads.mongodb.com/compass/mongodb-mongosh_2.1.1_amd64.deb"
            fi
            wget -q "$URL" -O /tmp/mongosh.deb
            sudo dpkg -i /tmp/mongosh.deb
            rm /tmp/mongosh.deb
            ;;
        mysql)
            log "[+] Installing MySQL client..."
            sudo apt-get update -qq && sudo apt-get install -y -qq default-mysql-client
            ;;
        kubectl)
            log "[+] Installing kubectl..."
            if [ "$ARCH" = "arm64" ]; then
                URL="https://dl.k8s.io/release/v1.29.0/bin/linux/arm64/kubectl"
            else
                URL="https://dl.k8s.io/release/v1.29.0/bin/linux/amd64/kubectl"
            fi
            curl -sLo /tmp/kubectl "$URL"
            sudo install -o root -g root -m 0755 /tmp/kubectl /usr/local/bin/kubectl
            rm /tmp/kubectl
            ;;
        helm)
            log "[+] Installing Helm..."
            if [ "$ARCH" = "arm64" ]; then
                URL="https://get.helm.sh/helm-v3.14.0-linux-arm64.tar.gz"
                DIR="linux-arm64"
            else
                URL="https://get.helm.sh/helm-v3.14.0-linux-amd64.tar.gz"
                DIR="linux-amd64"
            fi
            wget -q "$URL" -O /tmp/helm.tar.gz
            tar -xzf /tmp/helm.tar.gz -C /tmp
            sudo install -o root -g root -m 0755 /tmp/$DIR/helm /usr/local/bin/helm
            rm -rf /tmp/helm* /tmp/linux-*
            ;;
        terraform)
            log "[+] Installing Terraform..."
            if [ "$ARCH" = "arm64" ]; then
                URL="https://releases.hashicorp.com/terraform/1.7.0/terraform_1.7.0_linux_arm64.zip"
            else
                URL="https://releases.hashicorp.com/terraform/1.7.0/terraform_1.7.0_linux_amd64.zip"
            fi
            wget -q "$URL" -O /tmp/terraform.zip
            unzip -q /tmp/terraform.zip -d /tmp
            sudo install -o root -g root -m 0755 /tmp/terraform /usr/local/bin/terraform
            rm -rf /tmp/terraform*
            ;;
        aws)
            log "[+] Installing AWS CLI..."
            if [ "$ARCH" = "arm64" ]; then
                URL="https://awscli.amazonaws.com/awscli-exe-linux-aarch64.zip"
            else
                URL="https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip"
            fi
            curl -s "$URL" -o /tmp/awscliv2.zip
            unzip -q /tmp/awscliv2.zip -d /tmp
            sudo /tmp/aws/install --update 2>/dev/null || sudo /tmp/aws/install
            rm -rf /tmp/aws*
            ;;
        heroku)
            log "[+] Installing Heroku CLI..."
            npm install -g heroku
            ;;
        vercel)
            log "[+] Installing Vercel CLI..."
            npm install -g vercel
            ;;
        netlify)
            log "[+] Installing Netlify CLI..."
            npm install -g netlify-cli
            ;;
        flyctl|fly)
            log "[+] Installing Fly.io CLI..."
            curl -sL https://fly.io/install.sh | sh
            sudo ln -sf ~/.fly/bin/flyctl /usr/local/bin/fly
            sudo ln -sf ~/.fly/bin/flyctl /usr/local/bin/flyctl
            ;;
        *)
            log "[!] Unknown tool: $tool (skipping)"
            return 1
            ;;
    esac

    # Mark as installed
    echo "$tool" >> "$TOOLS_INSTALLED"
    log "[+] Tool '$tool' installed successfully"
}

# Parse tools from hive.yaml
if [ -f "/hive-config/hive.yaml" ] && command -v yq &> /dev/null; then
    TOOLS=$(yq -r '.tools[]? // empty' /hive-config/hive.yaml 2>/dev/null)

    if [ -n "$TOOLS" ]; then
        log "[+] Installing CLI tools from hive.yaml..."
        for tool in $TOOLS; do
            install_tool "$tool"
        done
        log "[+] CLI tools installation complete"
    fi
fi

# ============================================
# Dependencies Installation with Smart Caching
# ============================================

# Auto-install npm/pnpm dependencies if package.json exists
if [ -f "$WORKSPACE_DIR/package.json" ] && [ "${AUTO_INSTALL_DEPS:-true}" = "true" ]; then
    # Check if node_modules is empty or missing (Docker volume creates empty dir)
    if [ ! -d "$WORKSPACE_DIR/node_modules" ] || [ -z "$(ls -A "$WORKSPACE_DIR/node_modules" 2>/dev/null)" ]; then
        cd "$WORKSPACE_DIR"

        # Generate cache key from package.json + lockfile (includes Node version for safety)
        NODE_VERSION=$(node --version)
        CACHE_KEY_INPUT="${NODE_VERSION}"

        # Add package.json to cache key
        if [ -f "package.json" ]; then
            CACHE_KEY_INPUT="${CACHE_KEY_INPUT}$(cat package.json)"
        fi

        # Add lockfile to cache key (pnpm-lock.yaml or package-lock.json)
        if [ -f "pnpm-lock.yaml" ]; then
            CACHE_KEY_INPUT="${CACHE_KEY_INPUT}$(cat pnpm-lock.yaml)"
            PKG_MANAGER="pnpm"
        elif [ -f "package-lock.json" ]; then
            CACHE_KEY_INPUT="${CACHE_KEY_INPUT}$(cat package-lock.json)"
            PKG_MANAGER="npm"
        else
            PKG_MANAGER="npm"
        fi

        # Generate SHA256 hash for cache key
        CACHE_KEY=$(echo -n "$CACHE_KEY_INPUT" | sha256sum | cut -d' ' -f1)
        CACHE_DIR="/home/agent/node_modules_cache/${CACHE_KEY}"

        log "[+] Dependency cache key: ${CACHE_KEY:0:12}..."

        # Check if cache exists
        if [ -d "$CACHE_DIR" ]; then
            log "[+] 🎯 Cache HIT! Restoring cached node_modules..."
            cp -r "$CACHE_DIR" "$WORKSPACE_DIR/node_modules"

            log "[+] Verifying dependencies with --prefer-offline..."
            if [ "$PKG_MANAGER" = "pnpm" ]; then
                pnpm install --prefer-offline --frozen-lockfile 2>&1 | grep -v "deprecated" | head -20
            elif [ "$PKG_MANAGER" = "npm" ]; then
                npm ci --prefer-offline 2>&1 | grep -v "deprecated" | head -20
            fi

            log "[+] ✅ Dependencies verified from cache (~10-30s)"
        else
            log "[+] ❌ Cache MISS. Installing dependencies fresh..."

            if [ "$PKG_MANAGER" = "pnpm" ]; then
                pnpm install --frozen-lockfile 2>&1 | grep -v "deprecated" | head -20
            elif [ "$PKG_MANAGER" = "npm" ]; then
                npm ci 2>&1 | grep -v "deprecated" | head -20
            else
                npm install 2>&1 | grep -v "deprecated" | head -20
            fi

            log "[+] Caching node_modules for future use..."
            mkdir -p "$(dirname "$CACHE_DIR")"
            cp -r "$WORKSPACE_DIR/node_modules" "$CACHE_DIR"

            log "[+] ✅ Dependencies installed and cached (~2-3min first time)"
        fi
    else
        log "[+] node_modules already exists, skipping install"
    fi
fi

# ============================================
# Final Setup
# ============================================

log "✅ Agent ${AGENT_ID} ready!"
log ""
log "Workspace: $WORKSPACE_DIR"
log "Role: ${AGENT_ROLE:-worker}"
log "Model: ${CLAUDE_MODEL:-sonnet}"
log ""

# ============================================
# Terminal Title Configuration (xterm OSC standard)
# ============================================

# Detect agent role and set appropriate emoji and title
if [ "$AGENT_ROLE" = "orchestrator" ]; then
    TERMINAL_TITLE="👑 Hive Queen"
else
    # Extract drone number from AGENT_ID (e.g., "drone-1" -> "1")
    DRONE_NUM="${AGENT_ID#drone-}"
    TERMINAL_TITLE="🐝 Hive Drone-${DRONE_NUM}"
fi

# Set terminal title using xterm OSC standard sequence
# OSC 0 = Change both window and tab title (most portable)
# Works on iTerm2, Terminal.app, Alacritty, tmux, etc.
printf '\033]0;%s\007' "$TERMINAL_TITLE"
log "[+] Terminal title: $TERMINAL_TITLE"

# ============================================
# Automatic Daemon Launch for Workers
# ============================================

# Check if worker should run in daemon mode
if [ "$AGENT_ROLE" = "worker" ] && [ -f "/home/agent/start-worker.sh" ]; then
    # start-worker.sh will check WORKER_N_MODE and launch daemon or bash
    exec /home/agent/start-worker.sh
else
    # Queen or interactive worker: execute command or start bash
    exec "$@"
fi
