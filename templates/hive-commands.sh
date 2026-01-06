#!/bin/bash
# Hive Agent Commands
# These commands are available to agents for communicating with the Hub

# Configuration (set by agent spawner)
HIVE_HUB_URL="${HIVE_HUB_URL:-http://localhost:8080}"
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

# hive-help: Show help
hive-help() {
    cat << 'EOF'
Hive Agent Commands
===================

Task Management:
  hive-task              Show current task
  hive-step              Show current step
  hive-complete '<json>' Mark task as complete
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

Examples:
  hive-solicit '{"type": "decision", "message": "Which option?"}'
  hive-port acquire 3000 --service=frontend
  hive-complete '{"result": "Feature implemented"}'

EOF
}

# Export functions
export -f hive-task hive-step hive-solicit hive-progress hive-complete hive-fail hive-port hive-status hive-help _hive_api
