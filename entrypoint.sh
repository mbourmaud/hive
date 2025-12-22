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

# ~/.claude is now SHARED across all agents (mounted from host)
# Only history.jsonl and session-env are ISOLATED per agent

log "[+] Claude configuration: SHARED (MCPs, skills, settings)"
log "[+] Conversation history: ISOLATED (this agent only)"

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

# Initialize workspace if it doesn't exist
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

# Execute command or start bash
exec "$@"
