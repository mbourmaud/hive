#!/usr/bin/env python3
"""
Claude Backend Implementations for Hive Worker Daemon

Supports multiple authentication methods:
- Anthropic API Key (direct SDK)
- Claude CLI with OAuth Token (subprocess)
- AWS Bedrock (enterprise)
"""

import os
import sys
import json
import logging
import subprocess
from abc import ABC, abstractmethod
from typing import Dict, Any, List, Optional, Callable

logger = logging.getLogger(__name__)

# Type alias for event callback
EventCallback = Callable[[Dict[str, Any]], None]


class ClaudeBackend(ABC):
    """Abstract base class for Claude API backends"""

    def __init__(self, model: str):
        self.model = model

    @abstractmethod
    def send_message(
        self,
        messages: List[Dict[str, Any]],
        system: str,
        max_tokens: int = 4096,
        tools: Optional[List[Dict[str, Any]]] = None,
        on_event: Optional[EventCallback] = None
    ) -> Dict[str, Any]:
        """
        Send a message to Claude and get response.

        Args:
            messages: List of message dicts with 'role' and 'content'
            system: System prompt
            max_tokens: Maximum tokens in response
            tools: Optional list of tool definitions
            on_event: Optional callback for streaming events (CLI backend only)

        Returns:
            Response dict with 'content', 'stop_reason', etc.
        """
        pass

    @abstractmethod
    def get_backend_name(self) -> str:
        """Return human-readable backend name"""
        pass


class AnthropicAPIBackend(ClaudeBackend):
    """Backend using Anthropic API with API key"""

    def __init__(self, api_key: str, model: str):
        super().__init__(model)
        try:
            from anthropic import Anthropic
            self.client = Anthropic(api_key=api_key)
            logger.info("✓ Initialized Anthropic API backend")
        except ImportError:
            logger.error("anthropic package not installed. Run: pip install anthropic")
            sys.exit(1)

    def send_message(
        self,
        messages: List[Dict[str, Any]],
        system: str,
        max_tokens: int = 4096,
        tools: Optional[List[Dict[str, Any]]] = None,
        on_event: Optional[EventCallback] = None
    ) -> Dict[str, Any]:
        """Send message via Anthropic SDK"""
        kwargs = {
            "model": self.model,
            "max_tokens": max_tokens,
            "system": system,
            "messages": messages
        }

        if tools:
            kwargs["tools"] = tools

        response = self.client.messages.create(**kwargs)

        return {
            "content": [{"type": block.type, "text": getattr(block, "text", "")}
                       for block in response.content],
            "stop_reason": response.stop_reason,
            "usage": {
                "input_tokens": response.usage.input_tokens,
                "output_tokens": response.usage.output_tokens
            }
        }

    def get_backend_name(self) -> str:
        return "Anthropic API (SDK)"


class ClaudeCLIBackend(ClaudeBackend):
    """Backend using Claude CLI with OAuth token (subprocess)"""

    def __init__(self, model: str):
        super().__init__(model)

        # Verify claude CLI is available
        try:
            result = subprocess.run(
                ["claude", "--version"],
                capture_output=True,
                text=True,
                timeout=5
            )
            if result.returncode != 0:
                raise RuntimeError("claude CLI not available")
            logger.info(f"✓ Initialized Claude CLI backend (version: {result.stdout.strip()})")
        except (subprocess.TimeoutExpired, FileNotFoundError, RuntimeError) as e:
            logger.error(f"claude CLI not available: {e}")
            logger.error("Make sure Claude Code is installed and accessible")
            sys.exit(1)

    def send_message(
        self,
        messages: List[Dict[str, Any]],
        system: str,
        max_tokens: int = 4096,
        tools: Optional[List[Dict[str, Any]]] = None,
        on_event: Optional[EventCallback] = None
    ) -> Dict[str, Any]:
        """
        Send message via Claude CLI subprocess with streaming JSON output.

        Note: This creates a new session for each call, so no conversation memory
        between tasks. Good for isolated task execution.
        """
        # Build prompt from messages
        prompt_parts = [system, "\n\n"]
        for msg in messages:
            role = msg["role"]
            content = msg["content"]
            if isinstance(content, list):
                # Extract text from content blocks
                content = " ".join([
                    block.get("text", "") for block in content
                    if block.get("type") == "text"
                ])
            prompt_parts.append(f"{role.upper()}: {content}\n\n")

        full_prompt = "".join(prompt_parts)

        # Call claude CLI with stream-json for real-time events
        try:
            cmd = [
                "claude",
                "-p",  # Print mode (non-interactive)
                "--verbose",  # Required for stream-json with -p
                "--output-format", "stream-json",  # Stream JSON events
                "--model", self._map_model_name(self.model),
                "--dangerously-skip-permissions",  # Skip permission prompts in daemon
                full_prompt
            ]

            # Use Popen for streaming output
            process = subprocess.Popen(
                cmd,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )

            final_text = ""
            events_collected = []

            # Read and process each line of JSON output
            for line in process.stdout:
                line = line.strip()
                if not line:
                    continue

                try:
                    event = json.loads(line)
                    events_collected.append(event)

                    # Fire callback if provided
                    if on_event:
                        on_event(event)

                    # Collect final response text
                    event_type = event.get("type")
                    if event_type == "assistant":
                        # Final assistant message
                        message = event.get("message", {})
                        content = message.get("content", [])
                        for block in content:
                            if block.get("type") == "text":
                                final_text = block.get("text", "")
                    elif event_type == "result":
                        # Result event contains the final text
                        result_text = event.get("result", "")
                        if result_text:
                            final_text = result_text

                except json.JSONDecodeError:
                    # Not JSON, might be plain text fallback
                    final_text += line + "\n"

            # Wait for process to complete
            process.wait()

            if process.returncode != 0:
                stderr = process.stderr.read() if process.stderr else ""
                logger.error(f"Claude CLI error (code {process.returncode}): {stderr}")
                raise RuntimeError(f"Claude CLI failed: {stderr}")

            return {
                "content": [{"type": "text", "text": final_text.strip()}],
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": 0,  # CLI doesn't provide token counts
                    "output_tokens": 0
                }
            }

        except Exception as e:
            logger.error(f"Claude CLI execution failed: {e}")
            raise

    def _map_model_name(self, model: str) -> str:
        """Map internal model names to Claude CLI model names"""
        model_map = {
            "claude-sonnet-4-20250514": "sonnet",
            "claude-opus-4-20250514": "opus",
            "claude-3-5-sonnet-20241022": "sonnet",
            "claude-3-opus-20240229": "opus"
        }
        return model_map.get(model, "sonnet")

    def get_backend_name(self) -> str:
        return "Claude CLI (OAuth)"


class BedrockBackend(ClaudeBackend):
    """Backend using AWS Bedrock"""

    def __init__(self, model: str, region: str = "us-east-1", profile: Optional[str] = None):
        super().__init__(model)
        self.region = region
        self.profile = profile

        try:
            import boto3

            session_kwargs = {"region_name": region}
            if profile:
                session_kwargs["profile_name"] = profile

            session = boto3.Session(**session_kwargs)
            self.client = session.client("bedrock-runtime")

            logger.info(f"✓ Initialized AWS Bedrock backend (region: {region})")
        except ImportError:
            logger.error("boto3 not installed. Run: pip install boto3")
            sys.exit(1)
        except Exception as e:
            logger.error(f"Failed to initialize Bedrock: {e}")
            sys.exit(1)

    def send_message(
        self,
        messages: List[Dict[str, Any]],
        system: str,
        max_tokens: int = 4096,
        tools: Optional[List[Dict[str, Any]]] = None,
        on_event: Optional[EventCallback] = None
    ) -> Dict[str, Any]:
        """Send message via AWS Bedrock"""

        # Format request for Bedrock
        request_body = {
            "anthropic_version": "bedrock-2023-05-31",
            "max_tokens": max_tokens,
            "system": system,
            "messages": messages
        }

        if tools:
            request_body["tools"] = tools

        # Map model name to Bedrock model ID
        bedrock_model_id = self._get_bedrock_model_id(self.model)

        try:
            response = self.client.invoke_model(
                modelId=bedrock_model_id,
                body=json.dumps(request_body)
            )

            response_body = json.loads(response["body"].read())

            return {
                "content": response_body.get("content", []),
                "stop_reason": response_body.get("stop_reason"),
                "usage": response_body.get("usage", {})
            }

        except Exception as e:
            logger.error(f"Bedrock API error: {e}")
            raise

    def _get_bedrock_model_id(self, model: str) -> str:
        """Map Anthropic model names to Bedrock model IDs"""
        model_map = {
            "claude-sonnet-4-20250514": "anthropic.claude-3-5-sonnet-20241022-v2:0",
            "claude-opus-4-20250514": "anthropic.claude-opus-4-20250514-v1:0",
            "claude-3-5-sonnet-20241022": "anthropic.claude-3-5-sonnet-20241022-v2:0",
            "claude-3-opus-20240229": "anthropic.claude-3-opus-20240229-v1:0"
        }
        return model_map.get(model, "anthropic.claude-3-5-sonnet-20241022-v2:0")

    def get_backend_name(self) -> str:
        return f"AWS Bedrock ({self.region})"


def create_backend(model: str = "claude-sonnet-4-20250514") -> ClaudeBackend:
    """
    Auto-detect and create appropriate Claude backend based on environment.

    Detection order:
    1. Check HIVE_CLAUDE_BACKEND env var for explicit choice
    2. Check ANTHROPIC_API_KEY for API backend
    3. Check AWS_PROFILE or CLAUDE_CODE_USE_BEDROCK for Bedrock
    4. Fall back to Claude CLI (requires CLAUDE_CODE_OAUTH_TOKEN)

    Args:
        model: Claude model to use

    Returns:
        Initialized ClaudeBackend instance

    Raises:
        RuntimeError: If no valid backend configuration found
    """

    # Allow explicit backend selection
    explicit_backend = os.getenv("HIVE_CLAUDE_BACKEND")

    if explicit_backend == "api":
        api_key = os.getenv("ANTHROPIC_API_KEY")
        if not api_key:
            raise RuntimeError("HIVE_CLAUDE_BACKEND=api but ANTHROPIC_API_KEY not set")
        logger.info("🔧 Using explicit backend: Anthropic API")
        return AnthropicAPIBackend(api_key=api_key, model=model)

    elif explicit_backend == "cli":
        oauth_token = os.getenv("CLAUDE_CODE_OAUTH_TOKEN")
        if not oauth_token:
            logger.warning("CLAUDE_CODE_OAUTH_TOKEN not set, but CLI should have cached auth")
        logger.info("🔧 Using explicit backend: Claude CLI")
        return ClaudeCLIBackend(model=model)

    elif explicit_backend == "bedrock":
        region = os.getenv("AWS_REGION", "us-east-1")
        profile = os.getenv("AWS_PROFILE")
        logger.info("🔧 Using explicit backend: AWS Bedrock")
        return BedrockBackend(model=model, region=region, profile=profile)

    # Auto-detection
    logger.info("🔍 Auto-detecting Claude backend...")

    # 1. Try API key first (most direct)
    api_key = os.getenv("ANTHROPIC_API_KEY")
    if api_key:
        logger.info("✓ Detected ANTHROPIC_API_KEY → using API backend")
        return AnthropicAPIBackend(api_key=api_key, model=model)

    # 2. Check for Bedrock
    if os.getenv("CLAUDE_CODE_USE_BEDROCK") == "1" or os.getenv("AWS_PROFILE"):
        region = os.getenv("AWS_REGION", "us-east-1")
        profile = os.getenv("AWS_PROFILE")
        logger.info(f"✓ Detected Bedrock config → using Bedrock backend (region: {region})")
        return BedrockBackend(model=model, region=region, profile=profile)

    # 3. Fall back to Claude CLI
    oauth_token = os.getenv("CLAUDE_CODE_OAUTH_TOKEN")
    if oauth_token or True:  # CLI might have cached auth
        logger.info("✓ Using Claude CLI backend (OAuth token)")
        return ClaudeCLIBackend(model=model)

    # No valid backend found
    raise RuntimeError(
        "No valid Claude backend configuration found.\n"
        "Please set one of:\n"
        "  - ANTHROPIC_API_KEY (for API access)\n"
        "  - CLAUDE_CODE_OAUTH_TOKEN (for CLI access)\n"
        "  - AWS_PROFILE + CLAUDE_CODE_USE_BEDROCK=1 (for Bedrock)\n"
        "Or explicitly set HIVE_CLAUDE_BACKEND=api|cli|bedrock"
    )
