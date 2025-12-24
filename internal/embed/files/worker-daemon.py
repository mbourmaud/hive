#!/usr/bin/env python3
"""
HIVE Worker Daemon - Autonomous task executor with full tool execution

This daemon polls Redis for tasks and executes them autonomously using Claude
with full tool execution (Read, Edit, Bash, Grep, Glob). It implements a complete
agentic loop where Claude can use tools to accomplish tasks.

Usage:
    python3 worker-daemon.py

Environment Variables:
    AGENT_ID: Worker ID (e.g., "drone-1")
    CLAUDE_CODE_OAUTH_TOKEN: Claude API authentication token
    REDIS_HOST: Redis host (default: "hive-redis")
    REDIS_PORT: Redis port (default: 6379)
    POLL_INTERVAL: Seconds between queue checks (default: 120)
    MAX_ITERATIONS: Maximum tool use iterations (default: 50)
"""

import os
import sys
import time
import json
import asyncio
import logging
from datetime import datetime
from typing import Optional, Dict, Any, List

# Add current directory to path for tools import
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

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

from tools import Tools, TOOL_DEFINITIONS, ToolExecutionError

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='[%(asctime)s] [%(levelname)s] %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)
logger = logging.getLogger(__name__)


class HiveWorkerDaemon:
    """Autonomous HIVE worker with full tool execution capabilities"""

    def __init__(self):
        # Get configuration from environment
        self.agent_id = os.getenv('AGENT_ID', 'drone-unknown')
        self.redis_host = os.getenv('REDIS_HOST', 'hive-redis')
        self.redis_port = int(os.getenv('REDIS_PORT', '6379'))
        self.poll_interval = int(os.getenv('POLL_INTERVAL', '120'))
        self.max_iterations = int(os.getenv('MAX_ITERATIONS', '50'))
        self.workspace = os.getenv('WORKSPACE_DIR', '/workspace')

        # Determine workspace name from path
        workspace_parts = self.workspace.rstrip('/').split('/')
        self.workspace_name = workspace_parts[-1] if workspace_parts else 'unknown'

        # Claude API setup
        api_key = os.getenv('CLAUDE_CODE_OAUTH_TOKEN') or os.getenv('ANTHROPIC_API_KEY')
        if not api_key:
            logger.error("No Claude API key found. Set CLAUDE_CODE_OAUTH_TOKEN or ANTHROPIC_API_KEY")
            sys.exit(1)

        self.client = Anthropic(api_key=api_key)
        self.model = os.getenv('CLAUDE_MODEL', 'claude-sonnet-4-20250514')

        # Tools instance
        self.tools = Tools(workspace=self.workspace)

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
        logger.info(f"📂 Workspace: {self.workspace} ({self.workspace_name})")
        logger.info(f"🤖 Model: {self.model}")
        logger.info(f"⏱️  Poll interval: {self.poll_interval}s")
        logger.info(f"🔧 Max iterations: {self.max_iterations}")

    def dequeue_task(self) -> Optional[Dict[str, Any]]:
        """Atomically dequeue a task from Redis queue to active list"""
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
            self.redis_client.rpop(self.active_key)
            completed_task = {
                **task,
                'status': 'completed',
                'completed_at': datetime.now().isoformat(),
                'result': result,
                'worker': self.agent_id
            }
            timestamp = time.time()
            self.redis_client.zadd('hive:completed', {json.dumps(completed_task): timestamp})
            task_id = task.get('id', 'unknown')
            self.redis_client.hset(f"hive:task:{task_id}", "data", json.dumps(completed_task))
            self.redis_client.publish(self.events_channel, f"task_completed:{self.agent_id}:{task_id}")
            logger.info(f"✅ Task completed: {task.get('title')}")
        except redis.RedisError as e:
            logger.error(f"Failed to mark task as completed: {e}")

    def mark_task_failed(self, task: Dict[str, Any], error: str):
        """Mark task as failed in Redis"""
        try:
            self.redis_client.rpop(self.active_key)
            failed_task = {
                **task,
                'status': 'failed',
                'failed_at': datetime.now().isoformat(),
                'error': error,
                'worker': self.agent_id
            }
            timestamp = time.time()
            self.redis_client.zadd('hive:failed', {json.dumps(failed_task): timestamp})
            task_id = task.get('id', 'unknown')
            self.redis_client.hset(f"hive:task:{task_id}", "data", json.dumps(failed_task))
            self.redis_client.publish(self.events_channel, f"task_failed:{self.agent_id}:{task_id}")
            logger.error(f"❌ Task failed: {task.get('title')} - {error}")
        except redis.RedisError as e:
            logger.error(f"Failed to mark task as failed: {e}")

    async def execute_task(self, task: Dict[str, Any]):
        """Execute task with full agentic loop and tool execution"""
        title = task.get('title', 'Untitled')
        description = task.get('description', '')
        branch = task.get('branch', 'main')
        jira_ticket = task.get('jira_ticket', '')

        # Build system prompt
        system_prompt = f"""You are Worker {self.agent_id} in the HIVE multi-agent system.

You have access to tools: read, edit, write, bash, grep, glob.
Use these tools to complete tasks autonomously.

WORKSPACE: {self.workspace} (project: {self.workspace_name})
BRANCH: {branch}
JIRA TICKET: {jira_ticket if jira_ticket else 'N/A'}

CRITICAL RULES:
1. Work autonomously - use tools without asking for permission
2. Read files before editing them
3. Run tests after making changes - CI MUST be GREEN
4. If tests fail, fix them before finishing
5. Commit changes with clear messages
6. Create PRs when appropriate
7. Provide a summary of what you accomplished

Use your tools effectively to complete the task."""

        # Build user message
        user_message = f"""TASK: {title}

DESCRIPTION:
{description}

Complete this task now using the available tools."""

        logger.info(f"🤖 Starting autonomous execution: {title}")

        # Agentic loop
        messages = [{"role": "user", "content": user_message}]
        iteration = 0
        final_response = ""

        try:
            while iteration < self.max_iterations:
                iteration += 1
                logger.debug(f"Iteration {iteration}/{self.max_iterations}")

                # Call Claude API with tools
                response = self.client.messages.create(
                    model=self.model,
                    max_tokens=8000,
                    system=system_prompt,
                    tools=TOOL_DEFINITIONS,
                    messages=messages
                )

                logger.debug(f"Stop reason: {response.stop_reason}")

                # Process response
                assistant_message = {"role": "assistant", "content": response.content}
                messages.append(assistant_message)

                # Check stop reason
                if response.stop_reason == "end_turn":
                    # Claude finished - extract final text
                    for block in response.content:
                        if hasattr(block, 'text'):
                            final_response += block.text
                    logger.info(f"✓ Task completed after {iteration} iterations")
                    break

                elif response.stop_reason == "tool_use":
                    # Execute tools
                    tool_results = []

                    for block in response.content:
                        if block.type == "tool_use":
                            tool_name = block.name
                            tool_input = block.input
                            tool_use_id = block.id

                            logger.info(f"🔧 Executing tool: {tool_name}")
                            logger.debug(f"Tool input: {json.dumps(tool_input, indent=2)}")

                            try:
                                # Execute the tool
                                result = self.tools.execute_tool(tool_name, tool_input)

                                # Format result
                                if isinstance(result, dict):
                                    result_str = json.dumps(result, indent=2)
                                elif isinstance(result, list):
                                    if len(result) == 0:
                                        result_str = "No results found"
                                    else:
                                        result_str = json.dumps(result[:10], indent=2)  # Limit list results
                                        if len(result) > 10:
                                            result_str += f"\n... and {len(result) - 10} more"
                                else:
                                    result_str = str(result)

                                logger.info(f"✓ Tool result: {result_str[:200]}...")

                                tool_results.append({
                                    "type": "tool_result",
                                    "tool_use_id": tool_use_id,
                                    "content": result_str
                                })

                            except ToolExecutionError as e:
                                error_msg = str(e)
                                logger.error(f"Tool execution failed: {error_msg}")

                                tool_results.append({
                                    "type": "tool_result",
                                    "tool_use_id": tool_use_id,
                                    "content": f"Error: {error_msg}",
                                    "is_error": True
                                })

                    # Add tool results to messages
                    messages.append({"role": "user", "content": tool_results})

                else:
                    # Unexpected stop reason
                    logger.warning(f"Unexpected stop reason: {response.stop_reason}")
                    break

            # Check if we hit max iterations
            if iteration >= self.max_iterations:
                raise Exception(f"Max iterations ({self.max_iterations}) reached")

            return final_response if final_response else "Task completed (no summary provided)"

        except Exception as e:
            error_msg = f"Execution failed: {str(e)}"
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
                await asyncio.sleep(10)

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
