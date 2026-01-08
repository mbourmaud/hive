#!/bin/bash
# Hive Agent Commands
# These commands are available to agents for communicating with the Hub

# Configuration (set by agent spawner)
HIVE_HUB_URL="${HIVE_HUB_URL:-http://localhost:7433}"
HIVE_AGENT_ID="${HIVE_AGENT_ID:-}"
HIVE_AGENT_NAME="${HIVE_AGENT_NAME:-}"

# Helper function for API calls
_hive_api() {
    local method="$1"
    local endpoint="$2"
    local data="$3"

    if [ -n "$data" ]; then
        curl -s -X "$method" "${HIVE_HUB_URL}${endpoint}" \
            -H "Content-Type: application/json" \
            -d "$data"
    else
        curl -s -X "$method" "${HIVE_HUB_URL}${endpoint}" \
            -H "Content-Type: application/json"
    fi
}

# hive-task: Show current task
hive-task() {
    if [ -z "$HIVE_AGENT_ID" ]; then
        echo "Error: HIVE_AGENT_ID not set"
        return 1
    fi

    _hive_api GET "/tasks?agent_id=${HIVE_AGENT_ID}&status=in_progress" | jq .
}

# hive-step: Show current step
hive-step() {
    if [ -z "$HIVE_AGENT_ID" ]; then
        echo "Error: HIVE_AGENT_ID not set"
        return 1
    fi

    local task=$(_hive_api GET "/tasks?agent_id=${HIVE_AGENT_ID}&status=in_progress" | jq '.[0]')

    if [ "$task" == "null" ] || [ -z "$task" ]; then
        echo "No active task"
        return 0
    fi

    local current_step=$(echo "$task" | jq -r '.current_step')
    echo "$task" | jq ".plan.steps[$current_step - 1]"
}

# hive-solicit: Create a solicitation to the Queen
hive-solicit() {
    local json="$1"

    if [ -z "$HIVE_AGENT_ID" ]; then
        echo "Error: HIVE_AGENT_ID not set"
        return 1
    fi

    if [ -z "$json" ]; then
        echo "Usage: hive-solicit '<json>'"
        echo "Example: hive-solicit '{\"type\": \"decision\", \"urgency\": \"medium\", \"message\": \"Which option?\"}'"
        return 1
    fi

    # Add agent info to the request
    local request=$(echo "$json" | jq --arg id "$HIVE_AGENT_ID" --arg name "$HIVE_AGENT_NAME" '. + {agent_id: $id, agent_name: $name}')

    _hive_api POST "/solicitations" "$request" | jq .
}

# hive-progress: Send a progress update
hive-progress() {
    local message="$1"

    if [ -z "$message" ]; then
        echo "Usage: hive-progress '<message>'"
        return 1
    fi

    hive-solicit "{\"type\": \"progress\", \"urgency\": \"low\", \"message\": \"$message\"}"
}

# hive-complete: Mark task as complete
hive-complete() {
    local json="${1:-{}}"

    if [ -z "$HIVE_AGENT_ID" ]; then
        echo "Error: HIVE_AGENT_ID not set"
        return 1
    fi

    # Get current task
    local task_id=$(_hive_api GET "/tasks?agent_id=${HIVE_AGENT_ID}&status=in_progress" | jq -r '.[0].id')

    if [ "$task_id" == "null" ] || [ -z "$task_id" ]; then
        echo "No active task to complete"
        return 1
    fi

    _hive_api POST "/tasks/${task_id}/complete" "$json" | jq .

    # Also create a completion solicitation
    local result=$(echo "$json" | jq -r '.result // "Task completed"')
    hive-solicit "{\"type\": \"completion\", \"urgency\": \"low\", \"message\": \"$result\"}"
}

# hive-fail: Mark task as failed
hive-fail() {
    local json="$1"

    if [ -z "$HIVE_AGENT_ID" ]; then
        echo "Error: HIVE_AGENT_ID not set"
        return 1
    fi

    if [ -z "$json" ]; then
        echo "Usage: hive-fail '{\"error\": \"Error message\"}'"
        return 1
    fi

    # Get current task
    local task_id=$(_hive_api GET "/tasks?agent_id=${HIVE_AGENT_ID}&status=in_progress" | jq -r '.[0].id')

    if [ "$task_id" == "null" ] || [ -z "$task_id" ]; then
        echo "No active task to fail"
        return 1
    fi

    _hive_api POST "/tasks/${task_id}/fail" "$json" | jq .

    # Also create a blocker solicitation
    local error=$(echo "$json" | jq -r '.error // "Task failed"')
    hive-solicit "{\"type\": \"blocker\", \"urgency\": \"high\", \"message\": \"$error\"}"
}

# hive-port: Manage port allocation
hive-port() {
    local action="$1"
    local port="$2"
    local service="${3:-default}"

    if [ -z "$HIVE_AGENT_ID" ]; then
        echo "Error: HIVE_AGENT_ID not set"
        return 1
    fi

    case "$action" in
        acquire)
            if [ -z "$port" ]; then
                echo "Usage: hive-port acquire <port> [--service=<name>]"
                return 1
            fi

            # Parse --service flag
            for arg in "$@"; do
                case "$arg" in
                    --service=*)
                        service="${arg#*=}"
                        ;;
                esac
            done

            local request="{\"agent_id\": \"${HIVE_AGENT_ID}\", \"agent_name\": \"${HIVE_AGENT_NAME}\", \"port\": ${port}, \"service\": \"${service}\", \"wait\": true}"

            echo "Acquiring port ${port}..."
            local response=$(_hive_api POST "/ports/acquire" "$request")
            local status=$(echo "$response" | jq -r '.status')

            case "$status" in
                acquired)
                    echo "Port ${port} acquired successfully"
                    return 0
                    ;;
                busy)
                    local held_by=$(echo "$response" | jq -r '.held_by.agent_name')
                    echo "Port ${port} is busy (held by ${held_by})"
                    return 1
                    ;;
                waiting)
                    echo "Waiting for port ${port}..."
                    # Wait will be handled by the server
                    return 0
                    ;;
                timeout)
                    echo "Timeout waiting for port ${port}"
                    return 1
                    ;;
                *)
                    echo "Error: $response"
                    return 1
                    ;;
            esac
            ;;

        release)
            if [ -z "$port" ]; then
                echo "Usage: hive-port release <port>"
                return 1
            fi

            local request="{\"agent_id\": \"${HIVE_AGENT_ID}\", \"port\": ${port}}"
            _hive_api POST "/ports/release" "$request" | jq .
            echo "Port ${port} released"
            ;;

        list)
            _hive_api GET "/ports" | jq .
            ;;

        status)
            if [ -z "$port" ]; then
                echo "Usage: hive-port status <port>"
                return 1
            fi
            _hive_api GET "/ports/${port}" | jq .
            ;;

        *)
            echo "Usage: hive-port <acquire|release|list|status> [args]"
            echo ""
            echo "Commands:"
            echo "  acquire <port> [--service=<name>]  Acquire a port"
            echo "  release <port>                     Release a port"
            echo "  list                               List all ports"
            echo "  status <port>                      Get port status"
            return 1
            ;;
    esac
}

# hive-status: Show agent status
hive-status() {
    if [ -z "$HIVE_AGENT_ID" ]; then
        echo "Error: HIVE_AGENT_ID not set"
        return 1
    fi

    echo "Agent: ${HIVE_AGENT_NAME} (${HIVE_AGENT_ID})"
    echo "Hub: ${HIVE_HUB_URL}"
    echo ""
    echo "Current Task:"
    hive-task
    echo ""
    echo "Ports:"
    _hive_api GET "/ports" | jq ".leases | map(select(.agent_id == \"${HIVE_AGENT_ID}\"))"
}

# hive-verify: Run verification checks (typecheck, test, build)
hive-verify() {
    local check_only="${1:-}"
    local failed=0
    local results=""

    echo "🔍 Running Ralph Loop verification..."
    echo ""

    # Detect package manager
    local pm="npm"
    if [ -f "pnpm-lock.yaml" ]; then
        pm="pnpm"
    elif [ -f "yarn.lock" ]; then
        pm="yarn"
    elif [ -f "bun.lockb" ]; then
        pm="bun"
    fi

    # Check if package.json exists
    if [ ! -f "package.json" ]; then
        echo "⚠️  No package.json found, skipping npm checks"
        echo ""
        
        # Try Go checks
        if [ -f "go.mod" ]; then
            echo "📦 Go project detected"
            echo ""
            
            echo "→ Running go vet..."
            if go vet ./... 2>&1; then
                echo "✅ go vet passed"
                results="${results}go_vet:pass,"
            else
                echo "❌ go vet failed"
                results="${results}go_vet:fail,"
                failed=1
            fi
            echo ""
            
            echo "→ Running go test..."
            if go test ./... 2>&1; then
                echo "✅ go test passed"
                results="${results}go_test:pass,"
            else
                echo "❌ go test failed"
                results="${results}go_test:fail,"
                failed=1
            fi
            echo ""
            
            echo "→ Running go build..."
            if go build ./... 2>&1; then
                echo "✅ go build passed"
                results="${results}go_build:pass,"
            else
                echo "❌ go build failed"
                results="${results}go_build:fail,"
                failed=1
            fi
        fi
    else
        # Node.js project
        echo "📦 Node.js project detected (using $pm)"
        echo ""

        # Typecheck
        if grep -q '"typecheck"' package.json 2>/dev/null || grep -q '"tsc"' package.json 2>/dev/null; then
            echo "→ Running typecheck..."
            if $pm run typecheck 2>&1; then
                echo "✅ Typecheck passed"
                results="${results}typecheck:pass,"
            else
                echo "❌ Typecheck failed"
                results="${results}typecheck:fail,"
                failed=1
            fi
            echo ""
        elif command -v tsc &> /dev/null && [ -f "tsconfig.json" ]; then
            echo "→ Running tsc --noEmit..."
            if tsc --noEmit 2>&1; then
                echo "✅ Typecheck passed"
                results="${results}typecheck:pass,"
            else
                echo "❌ Typecheck failed"
                results="${results}typecheck:fail,"
                failed=1
            fi
            echo ""
        fi

        # Test
        if grep -q '"test"' package.json 2>/dev/null; then
            echo "→ Running tests..."
            if $pm run test 2>&1; then
                echo "✅ Tests passed"
                results="${results}test:pass,"
            else
                echo "❌ Tests failed"
                results="${results}test:fail,"
                failed=1
            fi
            echo ""
        fi

        # Build
        if grep -q '"build"' package.json 2>/dev/null; then
            echo "→ Running build..."
            if $pm run build 2>&1; then
                echo "✅ Build passed"
                results="${results}build:pass,"
            else
                echo "❌ Build failed"
                results="${results}build:fail,"
                failed=1
            fi
            echo ""
        fi

        # Lint (optional, don't fail on this)
        if grep -q '"lint"' package.json 2>/dev/null; then
            echo "→ Running lint..."
            if $pm run lint 2>&1; then
                echo "✅ Lint passed"
                results="${results}lint:pass,"
            else
                echo "⚠️  Lint has warnings/errors (non-blocking)"
                results="${results}lint:warn,"
            fi
            echo ""
        fi
    fi

    # Summary
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    if [ $failed -eq 0 ]; then
        echo "✅ All verification checks passed!"
        echo ""
        echo "Ready to commit and mark task complete."
        return 0
    else
        echo "❌ Some verification checks failed!"
        echo ""
        echo "Fix the issues above and run hive-verify again."
        echo "Remember: Ralph Loop - iterate until ALL checks pass!"
        return 1
    fi
}

# hive-help: Show help
hive-help() {
    cat << 'EOF'
Hive Agent Commands (Ralph Loop Edition)
=========================================

Ralph Loop:
  hive-verify            Run all verification checks (typecheck, test, build)
                         MUST pass before marking task complete!

Task Management:
  hive-task              Show current task
  hive-step              Show current step
  hive-complete '<json>' Mark task as complete (run hive-verify first!)
  hive-fail '<json>'     Mark task as failed
  hive-progress '<msg>'  Send progress update

Communication with Queen:
  hive-solicit '<json>'  Create a solicitation

Port Management:
  hive-port acquire <port> [--service=<name>]  Acquire a port
  hive-port release <port>                     Release a port
  hive-port list                               List all ports
  hive-port status <port>                      Get port status

Status:
  hive-status            Show agent status
  hive-help              Show this help

Ralph Loop Workflow:
  1. Receive task
  2. Execute changes
  3. hive-verify          # Run checks
  4. If failed → fix and goto 3
  5. git commit
  6. hive-complete

Examples:
  hive-verify
  hive-solicit '{"type": "blocker", "message": "Build fails after 3 attempts"}'
  hive-port acquire 3000 --service=frontend
  hive-complete '{"result": "Feature implemented, all checks pass"}'

EOF
}

# Export functions
export -f hive-task hive-step hive-solicit hive-progress hive-complete hive-fail hive-port hive-status hive-help hive-verify _hive_api
