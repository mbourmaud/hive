#!/usr/bin/env python3
"""
Unit tests for Claude backends

Run with: python3 test_backends.py
or: python3 -m unittest test_backends.py
"""

import os
import sys
import unittest
from unittest.mock import Mock, patch, MagicMock
import json

# Add current directory to path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from backends import (
    ClaudeBackend,
    AnthropicAPIBackend,
    ClaudeCLIBackend,
    BedrockBackend,
    create_backend
)


class TestClaudeBackend(unittest.TestCase):
    """Test abstract base class"""

    def test_is_abstract(self):
        """ClaudeBackend should not be instantiable"""
        with self.assertRaises(TypeError):
            ClaudeBackend(model="test")


class TestAnthropicAPIBackend(unittest.TestCase):
    """Test Anthropic API backend"""

    def test_initialization(self):
        """Should initialize with API key"""
        with patch('anthropic.Anthropic') as mock_anthropic:
            backend = AnthropicAPIBackend(api_key="test-key", model="claude-sonnet-4")

            mock_anthropic.assert_called_once_with(api_key="test-key")
            self.assertEqual(backend.model, "claude-sonnet-4")
            self.assertEqual(backend.get_backend_name(), "Anthropic API (SDK)")

    def test_send_message(self):
        """Should send message via SDK"""
        with patch('anthropic.Anthropic') as mock_anthropic:
            # Setup mock
            mock_client = MagicMock()
            mock_anthropic.return_value = mock_client

            mock_response = MagicMock()
            mock_response.content = [
                MagicMock(type="text", text="Hello, world!")
            ]
            mock_response.stop_reason = "end_turn"
            mock_response.usage = MagicMock(input_tokens=100, output_tokens=50)

            mock_client.messages.create.return_value = mock_response

            # Test
            backend = AnthropicAPIBackend(api_key="test-key", model="claude-sonnet-4")
            response = backend.send_message(
                messages=[{"role": "user", "content": "test"}],
                system="test system",
                max_tokens=1000
            )

            # Verify
            mock_client.messages.create.assert_called_once()
            self.assertEqual(response['stop_reason'], "end_turn")
            self.assertEqual(response['content'][0]['text'], "Hello, world!")


class TestClaudeCLIBackend(unittest.TestCase):
    """Test Claude CLI backend"""

    @patch('subprocess.run')
    def test_initialization(self, mock_run):
        """Should verify claude CLI is available"""
        mock_run.return_value = MagicMock(returncode=0, stdout="claude v1.0.0\n")

        backend = ClaudeCLIBackend(model="claude-sonnet-4")

        self.assertEqual(backend.model, "claude-sonnet-4")
        self.assertEqual(backend.get_backend_name(), "Claude CLI (OAuth)")
        mock_run.assert_called_once()

    @patch('subprocess.run')
    def test_initialization_failure(self, mock_run):
        """Should fail if claude CLI not available"""
        mock_run.side_effect = FileNotFoundError()

        with self.assertRaises(SystemExit):
            ClaudeCLIBackend(model="claude-sonnet-4")

    @patch('subprocess.run')
    def test_send_message(self, mock_run):
        """Should send message via CLI subprocess"""
        # Mock version check
        version_result = MagicMock(returncode=0, stdout="claude v1.0.0\n")

        # Mock chat command
        chat_result = MagicMock(
            returncode=0,
            stdout="This is the response from Claude CLI\n",
            stderr=""
        )

        mock_run.side_effect = [version_result, chat_result]

        backend = ClaudeCLIBackend(model="claude-sonnet-4")
        response = backend.send_message(
            messages=[{"role": "user", "content": "test message"}],
            system="test system",
            max_tokens=1000
        )

        # Verify
        self.assertEqual(response['stop_reason'], "end_turn")
        self.assertEqual(response['content'][0]['text'], "This is the response from Claude CLI")
        self.assertEqual(mock_run.call_count, 2)  # version + chat

    @patch('subprocess.run')
    def test_model_mapping(self, mock_run):
        """Should map model names correctly"""
        mock_run.return_value = MagicMock(returncode=0, stdout="v1.0.0\n")

        backend = ClaudeCLIBackend(model="claude-sonnet-4-20250514")

        self.assertEqual(backend._map_model_name("claude-sonnet-4-20250514"), "sonnet")
        self.assertEqual(backend._map_model_name("claude-opus-4-20250514"), "opus")
        self.assertEqual(backend._map_model_name("unknown-model"), "sonnet")


class TestBedrockBackend(unittest.TestCase):
    """Test AWS Bedrock backend"""

    def test_initialization(self):
        """Should initialize with AWS credentials"""
        with patch('boto3.Session') as mock_session_class:
            mock_session = MagicMock()
            mock_session_class.return_value = mock_session

            backend = BedrockBackend(model="claude-sonnet-4", region="us-west-2", profile="test")

            self.assertEqual(backend.model, "claude-sonnet-4")
            self.assertEqual(backend.region, "us-west-2")
            self.assertEqual(backend.profile, "test")
            self.assertEqual(backend.get_backend_name(), "AWS Bedrock (us-west-2)")

            mock_session_class.assert_called_once_with(region_name="us-west-2", profile_name="test")
            mock_session.client.assert_called_once_with("bedrock-runtime")

    def test_send_message(self):
        """Should send message via Bedrock"""
        with patch('boto3.Session') as mock_session_class:
            mock_session = MagicMock()
            mock_client = MagicMock()
            mock_session_class.return_value = mock_session
            mock_session.client.return_value = mock_client

            # Mock Bedrock response
            bedrock_response = {
                "body": MagicMock(
                    read=lambda: json.dumps({
                        "content": [{"type": "text", "text": "Hello from Bedrock"}],
                        "stop_reason": "end_turn",
                        "usage": {"input_tokens": 100, "output_tokens": 50}
                    }).encode()
                )
            }
            mock_client.invoke_model.return_value = bedrock_response

            backend = BedrockBackend(model="claude-sonnet-4", region="us-east-1")
            response = backend.send_message(
                messages=[{"role": "user", "content": "test"}],
                system="test system",
                max_tokens=1000
            )

            # Verify
            self.assertEqual(response['stop_reason'], "end_turn")
            self.assertEqual(response['content'][0]['text'], "Hello from Bedrock")
            mock_client.invoke_model.assert_called_once()


class TestBackendFactory(unittest.TestCase):
    """Test create_backend factory function"""

    @patch.dict(os.environ, {"ANTHROPIC_API_KEY": "test-key"}, clear=True)
    def test_create_api_backend(self):
        """Should create API backend when API key present"""
        with patch('anthropic.Anthropic'):
            backend = create_backend(model="claude-sonnet-4")

            self.assertIsInstance(backend, AnthropicAPIBackend)
            self.assertEqual(backend.model, "claude-sonnet-4")

    @patch.dict(os.environ, {"HIVE_CLAUDE_BACKEND": "cli", "CLAUDE_CODE_OAUTH_TOKEN": "token"}, clear=True)
    @patch('subprocess.run')
    def test_create_cli_backend_explicit(self, mock_run):
        """Should create CLI backend when explicitly requested"""
        mock_run.return_value = MagicMock(returncode=0, stdout="v1.0.0\n")

        backend = create_backend(model="claude-sonnet-4")

        self.assertIsInstance(backend, ClaudeCLIBackend)

    @patch.dict(os.environ, {"HIVE_CLAUDE_BACKEND": "bedrock", "AWS_REGION": "us-west-2"}, clear=True)
    def test_create_bedrock_backend_explicit(self):
        """Should create Bedrock backend when explicitly requested"""
        with patch('boto3.Session'):
            backend = create_backend(model="claude-sonnet-4")

            self.assertIsInstance(backend, BedrockBackend)

    @patch.dict(os.environ, {"AWS_PROFILE": "bedrock"}, clear=True)
    def test_auto_detect_bedrock(self):
        """Should auto-detect Bedrock from AWS_PROFILE"""
        with patch('boto3.Session'):
            backend = create_backend(model="claude-sonnet-4")

            self.assertIsInstance(backend, BedrockBackend)

    @patch.dict(os.environ, {}, clear=True)
    @patch('subprocess.run')
    def test_fallback_to_cli(self, mock_run):
        """Should fall back to CLI when no other credentials present"""
        mock_run.return_value = MagicMock(returncode=0, stdout="v1.0.0\n")

        backend = create_backend(model="claude-sonnet-4")

        self.assertIsInstance(backend, ClaudeCLIBackend)


if __name__ == '__main__':
    unittest.main()
