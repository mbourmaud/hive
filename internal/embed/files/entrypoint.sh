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

if [ -n "$GIT_USER_EMAIL" ]; then
    git config --global user.email "$GIT_USER_EMAIL"
fi

if [ -n "$GIT_USER_NAME" ]; then
    git config --global user.name "$GIT_USER_NAME"
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
# Dependencies Installation (optional)
# ============================================

# Auto-install npm/pnpm dependencies if package.json exists
if [ -f "$WORKSPACE_DIR/package.json" ] && [ "${AUTO_INSTALL_DEPS:-true}" = "true" ]; then
    if [ ! -d "$WORKSPACE_DIR/node_modules" ]; then
        log "[+] Installing project dependencies..."
        cd "$WORKSPACE_DIR"

        # Use pnpm if pnpm-lock.yaml exists, otherwise npm
        if [ -f "pnpm-lock.yaml" ]; then
            pnpm install --frozen-lockfile 2>&1 | grep -v "deprecated"
        elif [ -f "package-lock.json" ]; then
            npm ci 2>&1 | grep -v "deprecated"
        else
            npm install 2>&1 | grep -v "deprecated"
        fi

        log "[+] Dependencies installed"
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
# Worker Mode Selection
# ============================================

if [ "$AGENT_ROLE" = "worker" ]; then
    WORKER_MODE="${WORKER_MODE:-interactive}"

    if [ "$WORKER_MODE" = "daemon" ]; then
        log "[+] Starting worker in DAEMON mode (autonomous)"
        log "[+] Polling Redis queue: hive:queue:${AGENT_ID}"

        # Check if daemon script exists
        if [ ! -f "/workspace/worker-daemon.py" ]; then
            log "[!] ERROR: worker-daemon.py not found in /workspace"
            log "[!] Falling back to interactive mode"
            exec bash
        fi

        # Execute daemon (replaces bash)
        exec python3 /workspace/worker-daemon.py
    else
        log "[+] Starting worker in INTERACTIVE mode"
        # Execute command or start bash (default behavior)
        exec "$@"
    fi
else
    # Queen/orchestrator - always interactive
    exec "$@"
fi
