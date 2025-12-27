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

# Push task to drone's queue
rcli LPUSH "hive:queue:$DRONE_ID" "$TASK_JSON"

# Publish notification
rcli PUBLISH "hive:events" "task_queued:$DRONE_ID"

echo "✅ Task queued for $DRONE_ID"
