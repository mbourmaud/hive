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

# Copy .claude/ from template (mounted read-only)
if [ -d /home/agent/.claude-template ]; then
    log "[+] Copying Claude configuration..."
    mkdir -p ~/.claude
    cp -r /home/agent/.claude-template/* ~/.claude/ 2>/dev/null || true
fi

# Setup isolated session data (mounted from workspace/.claude-data)
if [ -d /home/agent/.claude-data ]; then
    log "[+] Setting up isolated session data..."
    mkdir -p /home/agent/.claude-data/session-env

    # Symlink session-env directory
    if [ ! -L ~/.claude/session-env ]; then
        rm -rf ~/.claude/session-env 2>/dev/null || true
        ln -s /home/agent/.claude-data/session-env ~/.claude/session-env
    fi

    # Symlink history.jsonl
    if [ ! -L ~/.claude/history.jsonl ]; then
        rm -f ~/.claude/history.jsonl 2>/dev/null || true
        ln -s /home/agent/.claude-data/history.jsonl ~/.claude/history.jsonl
    fi

    # Persist .claude.json in .claude-data
    PERSISTENT_CONFIG="/home/agent/.claude-data/claude.json"

    # Initialize or update persistent config
    if [ ! -f "$PERSISTENT_CONFIG" ]; then
        log "[+] Creating persistent .claude.json from template..."
        if [ -f /home/agent/.claude.json.template ]; then
            cp /home/agent/.claude.json.template "$PERSISTENT_CONFIG"
        else
            echo '{}' > "$PERSISTENT_CONFIG"
        fi
    fi

    # Ensure template params are set
    if [ -f /home/agent/.claude.json.template ] && command -v jq &> /dev/null; then
        CLAUDE_VERSION=$(claude --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
        THEME="dark-high-contrast"
        if [ "$AGENT_ROLE" = "orchestrator" ]; then
            THEME="dark"
        fi

        # Merge template params into existing config
        jq --slurpfile template /home/agent/.claude.json.template \
           --arg version "${CLAUDE_VERSION:-2.0.75}" \
           --arg theme "$THEME" \
           --arg token "${CLAUDE_CODE_OAUTH_TOKEN:-}" \
           '
           # Merge template setup flags
           ($template[0] // {}) as $tpl |
           . + {
             version: $version,
             theme: $theme,
             auth: (.auth // {} | . + {oauthToken: $token})
           } +
           # Keep setup flags from template (showSetupPrompt, etc.)
           ($tpl | with_entries(select(.key | test("^(showSetupPrompt|bypassPermissionsAccepted)$"))))
           ' "$PERSISTENT_CONFIG" > "$PERSISTENT_CONFIG.tmp" && \
           mv "$PERSISTENT_CONFIG.tmp" "$PERSISTENT_CONFIG"
    fi

    # Symlink to persistent config
    if [ ! -L ~/.claude/claude.json ]; then
        rm -f ~/.claude/claude.json 2>/dev/null || true
        ln -s "$PERSISTENT_CONFIG" ~/.claude/claude.json
    fi
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
