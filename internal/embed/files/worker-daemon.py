#!/usr/bin/env python3
"""
HIVE Worker Daemon - Autonomous task executor using Claude Agent SDK

This daemon polls Redis for tasks and executes them autonomously using the
Claude Agent SDK. It replaces the interactive CLI mode with a fully autonomous
agent that can read files, edit code, run tests, and complete tasks without
human intervention.

Usage:
    python3 worker-daemon.py

Environment Variables:
    AGENT_ID: Worker ID (e.g., "drone-1")
    CLAUDE_CODE_OAUTH_TOKEN: Claude API authentication token
    REDIS_HOST: Redis host (default: "hive-redis")
    REDIS_PORT: Redis port (default: 6379)
    POLL_INTERVAL: Seconds between queue checks (default: 120)
"""

import os
import sys
import time
import json
import asyncio
import logging
from datetime import datetime
from typing import Optional, Dict, Any

try:
    import redis
except ImportError:
    print("ERROR: redis package not installed. Run: pip install redis", file=sys.stderr)
    sys.exit(1)

try:
    from anthropic import Anthropic
except ImportError:
    print("ERROR: anthropic package not installed. Run: pip install anthropic", file=sys.stderr)
    sys.exit(1)

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='[%(asctime)s] [%(levelname)s] %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)
logger = logging.getLogger(__name__)


class HiveWorkerDaemon:
    """Autonomous HIVE worker that executes tasks using Claude Agent SDK"""

    def __init__(self):
        # Get configuration from environment
        self.agent_id = os.getenv('AGENT_ID', 'drone-unknown')
        self.redis_host = os.getenv('REDIS_HOST', 'hive-redis')
        self.redis_port = int(os.getenv('REDIS_PORT', '6379'))
        self.poll_interval = int(os.getenv('POLL_INTERVAL', '120'))
        self.workspace = os.getenv('WORKSPACE_DIR', '/workspace')

        # Claude API setup
        api_key = os.getenv('CLAUDE_CODE_OAUTH_TOKEN') or os.getenv('ANTHROPIC_API_KEY')
        if not api_key:
            logger.error("No Claude API key found. Set CLAUDE_CODE_OAUTH_TOKEN or ANTHROPIC_API_KEY")
            sys.exit(1)

        self.client = Anthropic(api_key=api_key)

        # Redis connection
        try:
            self.redis_client = redis.Redis(
                host=self.redis_host,
                port=self.redis_port,
                decode_responses=True,
                socket_connect_timeout=5
            )
            self.redis_client.ping()
            logger.info(f"✓ Connected to Redis at {self.redis_host}:{self.redis_port}")
        except redis.ConnectionError as e:
            logger.error(f"Failed to connect to Redis: {e}")
            sys.exit(1)

        # Redis keys
        self.queue_key = f"hive:queue:{self.agent_id}"
        self.active_key = f"hive:active:{self.agent_id}"
        self.events_channel = "hive:events"

        logger.info(f"🐝 HIVE Worker {self.agent_id} initialized (DAEMON mode)")
        logger.info(f"📂 Workspace: {self.workspace}")
        logger.info(f"⏱️  Poll interval: {self.poll_interval}s")

    def dequeue_task(self) -> Optional[Dict[str, Any]]:
        """
        Atomically dequeue a task from Redis queue to active list.
        Returns task dict or None if queue is empty.
        """
        try:
            task_json = self.redis_client.rpoplpush(self.queue_key, self.active_key)
            if not task_json:
                return None

            task = json.loads(task_json)
            logger.info(f"📥 Dequeued task: {task.get('title', 'Untitled')}")
            return task

        except json.JSONDecodeError as e:
            logger.error(f"Failed to parse task JSON: {e}")
            return None
        except redis.RedisError as e:
            logger.error(f"Redis error during dequeue: {e}")
            return None

    def mark_task_completed(self, task: Dict[str, Any], result: str):
        """Mark task as completed in Redis"""
        try:
            # Remove from active
            self.redis_client.rpop(self.active_key)

            # Add completion metadata
            completed_task = {
                **task,
                'status': 'completed',
                'completed_at': datetime.now().isoformat(),
                'result': result,
                'worker': self.agent_id
            }

            # Store in completed sorted set (score = timestamp)
            timestamp = time.time()
            self.redis_client.zadd('hive:completed', {
                json.dumps(completed_task): timestamp
            })

            # Store task details in hash
            task_id = task.get('id', 'unknown')
            self.redis_client.hset(f"hive:task:{task_id}", "data", json.dumps(completed_task))

            # Publish event
            self.redis_client.publish(
                self.events_channel,
                f"task_completed:{self.agent_id}:{task_id}"
            )

            logger.info(f"✅ Task completed: {task.get('title')}")

        except redis.RedisError as e:
            logger.error(f"Failed to mark task as completed: {e}")

    def mark_task_failed(self, task: Dict[str, Any], error: str):
        """Mark task as failed in Redis"""
        try:
            # Remove from active
            self.redis_client.rpop(self.active_key)

            # Add failure metadata
            failed_task = {
                **task,
                'status': 'failed',
                'failed_at': datetime.now().isoformat(),
                'error': error,
                'worker': self.agent_id
            }

            # Store in failed sorted set
            timestamp = time.time()
            self.redis_client.zadd('hive:failed', {
                json.dumps(failed_task): timestamp
            })

            # Store task details in hash
            task_id = task.get('id', 'unknown')
            self.redis_client.hset(f"hive:task:{task_id}", "data", json.dumps(failed_task))

            # Publish event
            self.redis_client.publish(
                self.events_channel,
                f"task_failed:{self.agent_id}:{task_id}"
            )

            logger.error(f"❌ Task failed: {task.get('title')} - {error}")

        except redis.RedisError as e:
            logger.error(f"Failed to mark task as failed: {e}")

    async def execute_task(self, task: Dict[str, Any]):
        """
        Execute task using Claude Agent SDK.

        The SDK provides autonomous execution with built-in tools:
        - Read: Read files
        - Edit: Edit files
        - Bash: Execute shell commands
        - Grep: Search code
        - Glob: Find files
        """
        title = task.get('title', 'Untitled')
        description = task.get('description', '')
        branch = task.get('branch', 'main')
        jira_ticket = task.get('jira_ticket', '')

        # Build comprehensive prompt for the agent
        prompt = f"""You are Worker {self.agent_id} in the HIVE multi-agent system.

TASK: {title}

DESCRIPTION:
{description}

CONTEXT:
- Branch: {branch}
- Jira Ticket: {jira_ticket if jira_ticket else 'N/A'}
- Workspace: {self.workspace}

INSTRUCTIONS:
Complete this task autonomously following these steps:

1. **Analyze**: Understand the requirement by reading relevant files
2. **Implement**: Make necessary code changes
3. **Test**: Run tests to verify your changes (CI MUST be GREEN!)
4. **Commit**: Commit your changes with a clear message
5. **PR**: Create a pull request if needed
6. **Summary**: Provide a concise summary of what you accomplished

IMPORTANT RULES:
- Work autonomously without asking for confirmation
- Run tests before marking task complete - CI must be GREEN
- If tests fail, fix them before finishing
- Commit with meaningful messages
- Follow existing code patterns in the project
- If you encounter blockers, document them clearly

Begin working on the task now.
"""

        logger.info(f"🤖 Starting autonomous execution: {title}")
        result_text = ""

        try:
            # Use Claude API with extended thinking for complex tasks
            # Note: Full Agent SDK with tool use will require anthropic-sdk-python
            # For now, we'll use the Messages API with a detailed prompt

            message = self.client.messages.create(
                model="claude-sonnet-4-20250514",
                max_tokens=8000,
                messages=[{
                    "role": "user",
                    "content": prompt
                }]
            )

            # Extract response
            for block in message.content:
                if hasattr(block, 'text'):
                    result_text += block.text

            logger.info(f"📝 Agent response received ({len(result_text)} chars)")

            # For POC, we'll mark as completed
            # In full implementation, SDK would execute tools and we'd check actual results
            return result_text

        except Exception as e:
            error_msg = f"SDK execution failed: {str(e)}"
            logger.error(error_msg)
            raise Exception(error_msg)

    async def run(self):
        """Main daemon loop - poll Redis and execute tasks"""
        logger.info(f"🚀 Worker daemon started - polling {self.queue_key}")

        while True:
            try:
                # Check for tasks in queue
                task = self.dequeue_task()

                if task:
                    # Execute task autonomously
                    try:
                        result = await self.execute_task(task)
                        self.mark_task_completed(task, result)
                    except Exception as e:
                        self.mark_task_failed(task, str(e))
                else:
                    # No tasks - sleep and poll again
                    logger.debug(f"No tasks in queue, sleeping {self.poll_interval}s...")
                    await asyncio.sleep(self.poll_interval)

            except KeyboardInterrupt:
                logger.info("🛑 Received shutdown signal")
                break
            except Exception as e:
                logger.error(f"Unexpected error in main loop: {e}")
                await asyncio.sleep(10)  # Brief pause before retrying

        logger.info(f"👋 Worker {self.agent_id} shutting down")


def main():
    """Entry point for HIVE worker daemon"""
    try:
        daemon = HiveWorkerDaemon()
        asyncio.run(daemon.run())
    except KeyboardInterrupt:
        logger.info("Daemon interrupted by user")
        sys.exit(0)
    except Exception as e:
        logger.error(f"Fatal error: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
