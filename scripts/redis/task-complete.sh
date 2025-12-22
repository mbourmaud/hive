#!/bin/bash
# Mark active task as completed

set -e

DRONE_ID=$1
RESULT=${2:-"{}"}

if [ -z "$DRONE_ID" ]; then
    echo "Usage: $0 <drone-id> [result-json]"
    exit 1
fi

# Get task from active
TASK=$(redis-cli -h localhost -p 6380 RPOP "hive:active:$DRONE_ID")

if [ -z "$TASK" ] || [ "$TASK" = "(nil)" ]; then
    echo "No active task for $DRONE_ID"
    exit 1
fi

# Add completed timestamp and result
TASK_ID=$(echo "$TASK" | jq -r '.id // "unknown"')
COMPLETED_TASK=$(echo "$TASK" | jq ". + {status: \"completed\", completed_at: \"$(date -Iseconds)\", result: $RESULT}")

# Add to completed sorted set (score = timestamp)
TIMESTAMP=$(date +%s)
redis-cli -h localhost -p 6380 ZADD "hive:completed" "$TIMESTAMP" "$COMPLETED_TASK"

# Store task details in hash
redis-cli -h localhost -p 6380 HSET "hive:task:$TASK_ID" "data" "$COMPLETED_TASK"

# Publish notification
redis-cli -h localhost -p 6380 PUBLISH "hive:events" "task_completed:$DRONE_ID:$TASK_ID"

echo "✅ Task $TASK_ID completed"
