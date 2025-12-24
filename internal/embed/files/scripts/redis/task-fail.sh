#!/bin/bash
# Mark active task as failed

set -e

DRONE_ID=$1
ERROR=${2:-"Unknown error"}

if [ -z "$DRONE_ID" ]; then
    echo "Usage: $0 <drone-id> <error-message>"
    exit 1
fi

# Get task from active
TASK=$(redis-cli -h ${REDIS_HOST:-hive-redis} -p ${REDIS_PORT:-6379} RPOP "hive:active:$DRONE_ID")

if [ -z "$TASK" ] || [ "$TASK" = "(nil)" ]; then
    echo "No active task for $DRONE_ID"
    exit 1
fi

# Add failed timestamp and error
TASK_ID=$(echo "$TASK" | jq -r '.id // "unknown"')
FAILED_TASK=$(echo "$TASK" | jq ". + {status: \"failed\", failed_at: \"$(date -Iseconds)\", error: \"$ERROR\"}")

# Add to failed sorted set (score = timestamp)
TIMESTAMP=$(date +%s)
redis-cli -h ${REDIS_HOST:-hive-redis} -p ${REDIS_PORT:-6379} ZADD "hive:failed" "$TIMESTAMP" "$FAILED_TASK"

# Store task details in hash
redis-cli -h ${REDIS_HOST:-hive-redis} -p ${REDIS_PORT:-6379} HSET "hive:task:$TASK_ID" "data" "$FAILED_TASK"

# Publish notification
redis-cli -h ${REDIS_HOST:-hive-redis} -p ${REDIS_PORT:-6379} PUBLISH "hive:events" "task_failed:$DRONE_ID:$TASK_ID"

echo "❌ Task $TASK_ID failed: $ERROR"
