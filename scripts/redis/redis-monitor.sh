#!/bin/bash
# Real-time Redis monitoring for HIVE events

set -e

REDIS_HOST=${REDIS_HOST:-localhost}
REDIS_PORT=${REDIS_PORT:-6380}

echo "🔍 Monitoring HIVE Redis Events"
echo "Listening to hive:events channel..."
echo "Press Ctrl+C to stop"
echo ""

redis-cli -h $REDIS_HOST -p $REDIS_PORT SUBSCRIBE hive:events
