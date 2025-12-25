#!/usr/bin/env python3
"""
HIVE Worker Daemon - Autonomous task executor with full tool execution

This daemon polls Redis for tasks and executes them autonomously using Claude
with full tool execution (Read, Edit, Bash, Grep, Glob). It implements a complete
agentic loop where Claude can use tools to accomplish tasks.

Supports multiple Claude backends:
- Anthropic API (requires ANTHROPIC_API_KEY)
- Claude CLI with OAuth (requires CLAUDE_CODE_OAUTH_TOKEN, default for Claude Max/Pro)
- AWS Bedrock (requires AWS_PROFILE or AWS credentials)

Usage:
    python3 worker-daemon.py

Environment Variables:
    AGENT_ID: Worker ID (e.g., "drone-1")
    REDIS_HOST: Redis host (default: "hive-redis")
    REDIS_PORT: Redis port (default: 6379)
    POLL_INTERVAL: Seconds between queue checks (default: 1)
    MAX_ITERATIONS: Maximum tool use iterations (default: 50)

    Backend Configuration (auto-detected):
    HIVE_CLAUDE_BACKEND: Explicit backend choice (api|cli|bedrock)
    ANTHROPIC_API_KEY: API key for direct API access
    CLAUDE_CODE_OAUTH_TOKEN: OAuth token for CLI backend (Claude Max/Pro)
    AWS_PROFILE: AWS profile for Bedrock
    AWS_REGION: AWS region for Bedrock (default: us-east-1)
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

from backends import create_backend
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
        self.poll_interval = int(os.getenv('POLL_INTERVAL', '1'))
        self.max_iterations = int(os.getenv('MAX_ITERATIONS', '50'))
        self.workspace = os.getenv('WORKSPACE_DIR', '/workspace')

        # Determine workspace name from path
        workspace_parts = self.workspace.rstrip('/').split('/')
        self.workspace_name = workspace_parts[-1] if workspace_parts else 'unknown'

        # Claude backend setup (auto-detects API/CLI/Bedrock)
        self.model = os.getenv('CLAUDE_MODEL', 'claude-sonnet-4-20250514')
        self.backend = create_backend(model=self.model)

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
        self.logs_key = f"hive:logs:{self.agent_id}"
        self.current_task_id = None

        logger.info(f"🐝 HIVE Worker {self.agent_id} initialized (DAEMON mode)")
        logger.info(f"📂 Workspace: {self.workspace} ({self.workspace_name})")
        logger.info(f"🤖 Model: {self.model}")
        logger.info(f"🔌 Backend: {self.backend.get_backend_name()}")
        logger.info(f"⏱️ Poll interval: {self.poll_interval}s")
        logger.info(f"🔧 Max iterations: {self.max_iterations}")

    def log_activity(self, event_type: str, content: str, metadata: Optional[Dict] = None):
        """Log activity to Redis stream for Queen visibility"""
        try:
            entry = {
                'timestamp': datetime.now().isoformat(),
                'agent': self.agent_id,
                'task_id': self.current_task_id or 'none',
                'event': event_type,
                'content': content[:2000],  # Limit content size
            }
            if metadata:
                entry['metadata'] = json.dumps(metadata)

            # Add to agent-specific stream (capped at 1000 entries)
            self.redis_client.xadd(self.logs_key, entry, maxlen=1000)

            # Also add to global stream for Queen
            self.redis_client.xadd('hive:logs:all', entry, maxlen=5000)

            # Publish event for real-time subscribers
            self.redis_client.publish(f"hive:activity:{self.agent_id}", json.dumps(entry))

        except redis.RedisError as e:
            logger.warning(f"Failed to log to Redis: {e}")

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
        self.current_task_id = task.get('id', 'unknown')

        # Log task start
        self.log_activity('task_start', f"Starting: {title}", {'branch': branch, 'jira': jira_ticket})

        # Build system prompt
        system_prompt = f"""You are Worker {self.agent_id} in the HIVE multi-agent system.

## Available Tools

### Development Tools
- read: Read file contents
- edit: Edit files by string replacement
- write: Write/create files
- bash: Execute shell commands
- grep: Search for patterns in files
- glob: Find files by pattern

### HIVE Coordination Tools (USE THESE - NOT bash commands)
- hive_my_tasks: Check your task queue and active task
- hive_take_task: Take the next task from your queue
- hive_task_done: Mark current task as completed (result parameter optional)
- hive_task_failed: Mark current task as failed (error parameter required)
- hive_config: Read configuration from hive.yaml

IMPORTANT: Use the HIVE tools directly instead of bash commands for coordination.
Example: Use hive_my_tasks instead of running "my-tasks" in bash.

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
7. Use hive_task_done when finished (only if CI is GREEN)
8. Use hive_task_failed if you cannot complete the task

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

                # Call Claude backend with tools
                response = self.backend.send_message(
                    messages=messages,
                    system=system_prompt,
                    max_tokens=8000,
                    tools=TOOL_DEFINITIONS
                )

                logger.debug(f"Stop reason: {response['stop_reason']}")

                # Process response
                assistant_message = {"role": "assistant", "content": response['content']}
                messages.append(assistant_message)

                # Check stop reason
                if response['stop_reason'] == "end_turn":
                    # Claude finished - extract final text
                    for block in response['content']:
                        if isinstance(block, dict) and block.get('type') == 'text':
                            text = block.get('text', '')
                            final_response += text
                            if text.strip():
                                self.log_activity('claude_response', text)
                        elif hasattr(block, 'text'):
                            final_response += block.text
                            if block.text.strip():
                                self.log_activity('claude_response', block.text)
                    logger.info(f"✓ Task completed after {iteration} iterations")
                    self.log_activity('task_complete', f"Completed after {iteration} iterations")
                    break

                elif response['stop_reason'] == "tool_use":
                    # Execute tools
                    tool_results = []

                    for block in response['content']:
                        # Handle both dict (from backends) and object (from SDK)
                        block_type = block.get('type') if isinstance(block, dict) else getattr(block, 'type', None)

                        if block_type == "tool_use":
                            tool_name = block.get('name') if isinstance(block, dict) else block.name
                            tool_input = block.get('input') if isinstance(block, dict) else block.input
                            tool_use_id = block.get('id') if isinstance(block, dict) else block.id

                            logger.info(f"🔧 Executing tool: {tool_name}")
                            logger.debug(f"Tool input: {json.dumps(tool_input, indent=2)}")
                            self.log_activity('tool_call', f"{tool_name}", {'input': tool_input})

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
                                self.log_activity('tool_result', result_str[:500], {'tool': tool_name})

                                tool_results.append({
                                    "type": "tool_result",
                                    "tool_use_id": tool_use_id,
                                    "content": result_str
                                })

                            except ToolExecutionError as e:
                                error_msg = str(e)
                                logger.error(f"Tool execution failed: {error_msg}")
                                self.log_activity('tool_error', error_msg, {'tool': tool_name})

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
            self.log_activity('task_failed', error_msg)
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
