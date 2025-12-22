#!/bin/bash
# Show status of all tasks in the HIVE

set -e

REDIS_HOST=${REDIS_HOST:-localhost}
REDIS_PORT=${REDIS_PORT:-6380}

echo "=== HIVE Task Status ==="
echo ""

# Queued tasks
echo "📥 Queued Tasks:"
for key in $(redis-cli -h $REDIS_HOST -p $REDIS_PORT KEYS "hive:queue:*" 2>/dev/null | sort); do
    DRONE=$(echo $key | cut -d: -f3)
    COUNT=$(redis-cli -h $REDIS_HOST -p $REDIS_PORT LLEN "$key")
    if [ "$COUNT" -gt 0 ]; then
        echo "  $DRONE: $COUNT task(s)"
    fi
done

echo ""

# Active tasks
echo "⚡ Active Tasks:"
for key in $(redis-cli -h $REDIS_HOST -p $REDIS_PORT KEYS "hive:active:*" 2>/dev/null | sort); do
    DRONE=$(echo $key | cut -d: -f3)
    TASK=$(redis-cli -h $REDIS_HOST -p $REDIS_PORT LINDEX "$key" 0)
    if [ -n "$TASK" ] && [ "$TASK" != "(nil)" ]; then
        TITLE=$(echo "$TASK" | jq -r '.title // "Untitled"' 2>/dev/null || echo "Untitled")
        echo "  $DRONE: $TITLE"
    fi
done

echo ""

# Completed count
COMPLETED=$(redis-cli -h $REDIS_HOST -p $REDIS_PORT ZCARD "hive:completed" 2>/dev/null || echo 0)
echo "✅ Completed: $COMPLETED"

# Failed count
FAILED=$(redis-cli -h $REDIS_HOST -p $REDIS_PORT ZCARD "hive:failed" 2>/dev/null || echo 0)
echo "❌ Failed: $FAILED"
