#!/bin/bash
# Mark active task as completed

set -e

DRONE_ID=$1
RESULT=${2:-"{}"}

if [ -z "$DRONE_ID" ]; then
    echo "Usage: $0 <drone-id> [result-json]"
    exit 1
fi

REDIS_HOST=${REDIS_HOST:-hive-redis}
REDIS_PORT=${REDIS_PORT:-6379}
REDIS_AUTH=""
if [ -n "$REDIS_PASSWORD" ]; then
    REDIS_AUTH="-a $REDIS_PASSWORD"
fi

# Helper function for redis-cli with auth
rcli() {
    redis-cli -h $REDIS_HOST -p $REDIS_PORT $REDIS_AUTH "$@" 2>/dev/null
}

# Get task from active
TASK=$(rcli RPOP "hive:active:$DRONE_ID")

if [ -z "$TASK" ] || [ "$TASK" = "(nil)" ]; then
    echo "No active task for $DRONE_ID"
    exit 1
fi

# Add completed timestamp and result
TASK_ID=$(echo "$TASK" | jq -r '.id // "unknown"')
COMPLETED_TASK=$(echo "$TASK" | jq ". + {status: \"completed\", completed_at: \"$(date -Iseconds)\", result: $RESULT}")

# Add to completed sorted set (score = timestamp)
TIMESTAMP=$(date +%s)
rcli ZADD "hive:completed" "$TIMESTAMP" "$COMPLETED_TASK"

# Store task details in hash
rcli HSET "hive:task:$TASK_ID" "data" "$COMPLETED_TASK"

# Publish notification
rcli PUBLISH "hive:events" "task_completed:$DRONE_ID:$TASK_ID"

echo "✅ Task $TASK_ID completed"
