#!/bin/bash
# Dequeue a task for a specific drone (atomically move from queue to active)

set -e

DRONE_ID=$1

if [ -z "$DRONE_ID" ]; then
    echo "Usage: $0 <drone-id>"
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

# Atomically pop from queue and push to active
TASK=$(rcli RPOPLPUSH "hive:queue:$DRONE_ID" "hive:active:$DRONE_ID")

if [ -z "$TASK" ] || [ "$TASK" = "(nil)" ]; then
    echo "No tasks in queue for $DRONE_ID"
    exit 0
fi

echo "$TASK"
