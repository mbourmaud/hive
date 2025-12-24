#!/bin/bash
# Enqueue a task for a specific drone

set -e

DRONE_ID=$1
TASK_JSON=$2

if [ -z "$DRONE_ID" ] || [ -z "$TASK_JSON" ]; then
    echo "Usage: $0 <drone-id> <task-json>"
    echo "Example: $0 drone-1 '{\"title\":\"My task\"}'"
    exit 1
fi

# Push task to drone's queue
redis-cli -h localhost -p 6380 LPUSH "hive:queue:$DRONE_ID" "$TASK_JSON"

# Publish notification
redis-cli -h localhost -p 6380 PUBLISH "hive:events" "task_queued:$DRONE_ID"

echo "✅ Task queued for $DRONE_ID"
