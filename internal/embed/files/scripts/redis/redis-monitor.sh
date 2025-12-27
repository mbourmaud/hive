#!/bin/bash
# Real-time Redis monitoring for HIVE events

set -e

REDIS_HOST=${REDIS_HOST:-redis}
REDIS_PORT=${REDIS_PORT:-6379}
REDIS_AUTH=""
if [ -n "$REDIS_PASSWORD" ]; then
    REDIS_AUTH="-a $REDIS_PASSWORD"
fi

echo "🔍 Monitoring HIVE Redis Events"
echo "Listening to hive:events channel..."
echo "Press Ctrl+C to stop"
echo ""

redis-cli -h $REDIS_HOST -p $REDIS_PORT $REDIS_AUTH SUBSCRIBE hive:events
