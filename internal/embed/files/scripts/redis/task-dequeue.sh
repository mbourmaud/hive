#!/bin/bash
# Dequeue a task for a specific drone (atomically move from queue to active)

set -e

DRONE_ID=$1

if [ -z "$DRONE_ID" ]; then
    echo "Usage: $0 <drone-id>"
    exit 1
fi

# Atomically pop from queue and push to active
TASK=$(redis-cli -h localhost -p 6380 RPOPLPUSH "hive:queue:$DRONE_ID" "hive:active:$DRONE_ID")

if [ -z "$TASK" ] || [ "$TASK" = "(nil)" ]; then
    echo "No tasks in queue for $DRONE_ID"
    exit 0
fi

echo "$TASK"
