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

# Create minimal settings.json for permissions only
cat > ~/.claude/settings.json << 'EOF'
{
  "permissions": {
    "defaultMode": "bypassPermissions"
  }
}
EOF
chmod 600 ~/.claude/settings.json
log "[+] Created minimal settings.json for permissions"

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
# Dependencies Installation with Smart Caching
# ============================================

# Auto-install npm/pnpm dependencies if package.json exists
if [ -f "$WORKSPACE_DIR/package.json" ] && [ "${AUTO_INSTALL_DEPS:-true}" = "true" ]; then
    if [ ! -d "$WORKSPACE_DIR/node_modules" ]; then
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
