#!/bin/bash
# Show status of all tasks in the HIVE

set -e

REDIS_HOST=${REDIS_HOST:-localhost}
REDIS_PORT=${REDIS_PORT:-6380}
REDIS_AUTH=""
if [ -n "$REDIS_PASSWORD" ]; then
    REDIS_AUTH="-a $REDIS_PASSWORD"
fi

# Helper function for redis-cli with auth
rcli() {
    redis-cli -h $REDIS_HOST -p $REDIS_PORT $REDIS_AUTH "$@" 2>/dev/null
}

echo "=== HIVE Task Status ==="
echo ""

# Queued tasks
echo "📥 Queued Tasks:"
for key in $(rcli KEYS "hive:queue:*" | sort); do
    DRONE=$(echo $key | cut -d: -f3)
    COUNT=$(rcli LLEN "$key")
    if [ "$COUNT" -gt 0 ]; then
        echo "  $DRONE: $COUNT task(s)"
    fi
done

echo ""

# Active tasks
echo "⚡ Active Tasks:"
for key in $(rcli KEYS "hive:active:*" | sort); do
    DRONE=$(echo $key | cut -d: -f3)
    TASK=$(rcli LINDEX "$key" 0)
    if [ -n "$TASK" ] && [ "$TASK" != "(nil)" ]; then
        TITLE=$(echo "$TASK" | jq -r '.title // "Untitled"' 2>/dev/null || echo "Untitled")
        echo "  $DRONE: $TITLE"
    fi
done

echo ""

# Completed count
COMPLETED=$(rcli ZCARD "hive:completed" || echo 0)
echo "✅ Completed: $COMPLETED"

# Failed count
FAILED=$(rcli ZCARD "hive:failed" || echo 0)
echo "❌ Failed: $FAILED"
