#!/bin/bash

# ===========================================
# Worker Startup Script
# Launches worker in interactive or daemon mode
# ===========================================

# Get worker ID from AGENT_ID (e.g., "drone-1" -> "1")
WORKER_NUM="${AGENT_ID#drone-}"

# Get worker mode from environment (WORKER_N_MODE)
WORKER_MODE_VAR="WORKER_${WORKER_NUM}_MODE"
WORKER_MODE="${!WORKER_MODE_VAR:-interactive}"

echo "🐝 Worker ${AGENT_ID} starting in ${WORKER_MODE} mode..."

if [ "$WORKER_MODE" = "daemon" ]; then
    # Autonomous mode: run worker-daemon in loop
    echo "🤖 Starting autonomous daemon mode"
    echo "   Worker will poll Redis queue and execute tasks automatically"
    echo "   Press Ctrl+C to stop"
    echo ""

    # Run daemon in loop (restart on crash)
    while true; do
        if [ -f "/home/agent/worker-daemon.py" ]; then
            python3 /home/agent/worker-daemon.py
        else
            echo "❌ Error: worker-daemon.py not found"
            echo "   Expected: /home/agent/worker-daemon.py"
            echo "   Falling back to bash"
            exec bash
        fi

        EXIT_CODE=$?
        if [ $EXIT_CODE -eq 0 ]; then
            echo "✅ Daemon exited cleanly"
            break
        else
            echo "⚠️  Daemon crashed (exit code: $EXIT_CODE), restarting in 5s..."
            sleep 5
        fi
    done
else
    # Interactive mode: launch bash for manual control
    echo "🖥️  Starting interactive mode"
    echo "   Connect with: hive connect ${WORKER_NUM}"
    echo ""
    exec bash
fi
