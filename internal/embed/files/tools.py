#!/usr/bin/env python3
"""
Tool implementations for HIVE Worker Daemon

Provides Read, Edit, Bash, Grep, Glob tools that can be executed
by the autonomous worker daemon.
"""

import os
import subprocess
import json
import re
from typing import Optional, Dict, Any, List
from pathlib import Path
import glob as glob_module


class ToolExecutionError(Exception):
    """Raised when a tool execution fails"""
    pass


class Tools:
    """Collection of tools for autonomous task execution"""

    def __init__(self, workspace: str = "/workspace"):
        self.workspace = workspace

    def read(self, file_path: str, offset: int = 0, limit: Optional[int] = None) -> str:
        """
        Read a file from the workspace.

        Args:
            file_path: Relative or absolute path to file
            offset: Line number to start reading from (0-indexed)
            limit: Maximum number of lines to read

        Returns:
            File contents with line numbers (cat -n format)
        """
        # Handle absolute vs relative paths
        if not file_path.startswith('/'):
            full_path = os.path.join(self.workspace, file_path)
        else:
            full_path = file_path

        if not os.path.exists(full_path):
            raise ToolExecutionError(f"File not found: {file_path}")

        if not os.path.isfile(full_path):
            raise ToolExecutionError(f"Not a file: {file_path}")

        try:
            with open(full_path, 'r', encoding='utf-8', errors='replace') as f:
                lines = f.readlines()

            # Apply offset and limit
            if offset > 0:
                lines = lines[offset:]
            if limit is not None:
                lines = lines[:limit]

            # Format with line numbers (cat -n format, starting from offset+1)
            result = ""
            for i, line in enumerate(lines):
                line_num = offset + i + 1
                # Truncate very long lines
                if len(line) > 2000:
                    line = line[:2000] + "...[truncated]\n"
                result += f"{line_num:6d}\t{line}"

            return result

        except Exception as e:
            raise ToolExecutionError(f"Error reading {file_path}: {str(e)}")

    def edit(self, file_path: str, old_string: str, new_string: str, replace_all: bool = False) -> str:
        """
        Edit a file by replacing exact string matches.

        Args:
            file_path: Relative or absolute path to file
            old_string: Exact string to find and replace
            new_string: Replacement string
            replace_all: If True, replace all occurrences; if False, require unique match

        Returns:
            Success message
        """
        # Handle absolute vs relative paths
        if not file_path.startswith('/'):
            full_path = os.path.join(self.workspace, file_path)
        else:
            full_path = file_path

        if not os.path.exists(full_path):
            raise ToolExecutionError(f"File not found: {file_path}")

        try:
            with open(full_path, 'r', encoding='utf-8') as f:
                content = f.read()

            # Check if old_string exists
            if old_string not in content:
                raise ToolExecutionError(f"String not found in {file_path}: {old_string[:50]}...")

            # Count occurrences
            count = content.count(old_string)

            if not replace_all and count > 1:
                raise ToolExecutionError(
                    f"String appears {count} times in {file_path}. "
                    f"Provide more context or use replace_all=true"
                )

            # Perform replacement
            new_content = content.replace(old_string, new_string)

            # Write back
            with open(full_path, 'w', encoding='utf-8') as f:
                f.write(new_content)

            return f"✓ Replaced {count} occurrence(s) in {file_path}"

        except ToolExecutionError:
            raise
        except Exception as e:
            raise ToolExecutionError(f"Error editing {file_path}: {str(e)}")

    def write(self, file_path: str, content: str) -> str:
        """
        Write content to a file (creates or overwrites).

        Args:
            file_path: Relative or absolute path to file
            content: Content to write

        Returns:
            Success message
        """
        # Handle absolute vs relative paths
        if not file_path.startswith('/'):
            full_path = os.path.join(self.workspace, file_path)
        else:
            full_path = file_path

        try:
            # Create parent directories if needed
            os.makedirs(os.path.dirname(full_path), exist_ok=True)

            # Write file
            with open(full_path, 'w', encoding='utf-8') as f:
                f.write(content)

            return f"✓ Wrote {len(content)} bytes to {file_path}"

        except Exception as e:
            raise ToolExecutionError(f"Error writing {file_path}: {str(e)}")

    def bash(self, command: str, timeout: int = 120) -> Dict[str, Any]:
        """
        Execute a bash command in the workspace.

        Args:
            command: Shell command to execute
            timeout: Maximum execution time in seconds

        Returns:
            Dict with stdout, stderr, exit_code
        """
        try:
            result = subprocess.run(
                command,
                shell=True,
                cwd=self.workspace,
                capture_output=True,
                text=True,
                timeout=timeout,
                env={**os.environ, 'HOME': '/home/agent'}
            )

            return {
                'stdout': result.stdout,
                'stderr': result.stderr,
                'exit_code': result.returncode,
                'success': result.returncode == 0
            }

        except subprocess.TimeoutExpired:
            raise ToolExecutionError(f"Command timed out after {timeout}s: {command}")
        except Exception as e:
            raise ToolExecutionError(f"Error executing command: {str(e)}")

    def grep(self, pattern: str, path: str = ".", glob_pattern: str = "*",
             case_insensitive: bool = False, context_lines: int = 0) -> List[Dict[str, Any]]:
        """
        Search for pattern in files.

        Args:
            pattern: Regex pattern to search for
            path: Directory or file to search in
            glob_pattern: File pattern to match (e.g., "*.py")
            case_insensitive: Case-insensitive search
            context_lines: Number of context lines to show

        Returns:
            List of matches with file, line_number, line content
        """
        # Handle absolute vs relative paths
        if not path.startswith('/'):
            full_path = os.path.join(self.workspace, path)
        else:
            full_path = path

        if not os.path.exists(full_path):
            raise ToolExecutionError(f"Path not found: {path}")

        matches = []
        flags = re.IGNORECASE if case_insensitive else 0

        try:
            pattern_re = re.compile(pattern, flags)
        except re.error as e:
            raise ToolExecutionError(f"Invalid regex pattern: {str(e)}")

        # Collect files to search
        files_to_search = []
        if os.path.isfile(full_path):
            files_to_search = [full_path]
        else:
            # Use glob to find files
            search_pattern = os.path.join(full_path, "**", glob_pattern)
            files_to_search = glob_module.glob(search_pattern, recursive=True)
            files_to_search = [f for f in files_to_search if os.path.isfile(f)]

        # Search in each file
        for file_path in files_to_search[:100]:  # Limit to 100 files
            try:
                with open(file_path, 'r', encoding='utf-8', errors='replace') as f:
                    lines = f.readlines()

                for i, line in enumerate(lines):
                    if pattern_re.search(line):
                        rel_path = os.path.relpath(file_path, self.workspace)
                        matches.append({
                            'file': rel_path,
                            'line_number': i + 1,
                            'line': line.rstrip(),
                            'context_before': lines[max(0, i-context_lines):i] if context_lines > 0 else [],
                            'context_after': lines[i+1:min(len(lines), i+1+context_lines)] if context_lines > 0 else []
                        })

            except Exception:
                # Skip files that can't be read
                continue

        return matches[:50]  # Limit results

    def glob(self, pattern: str, path: str = ".") -> List[str]:
        """
        Find files matching a glob pattern.

        Args:
            pattern: Glob pattern (e.g., "**/*.py", "src/*.js")
            path: Base directory to search in

        Returns:
            List of matching file paths
        """
        # Handle absolute vs relative paths
        if not path.startswith('/'):
            full_path = os.path.join(self.workspace, path)
        else:
            full_path = path

        if not os.path.exists(full_path):
            raise ToolExecutionError(f"Path not found: {path}")

        try:
            search_pattern = os.path.join(full_path, pattern)
            files = glob_module.glob(search_pattern, recursive=True)

            # Return relative paths
            result = [os.path.relpath(f, self.workspace) for f in files]
            return sorted(result)[:100]  # Limit to 100 results

        except Exception as e:
            raise ToolExecutionError(f"Error globbing {pattern}: {str(e)}")

    def execute_tool(self, tool_name: str, tool_input: Dict[str, Any]) -> Any:
        """
        Execute a tool by name with the given input.

        Args:
            tool_name: Name of tool to execute
            tool_input: Dictionary of tool parameters

        Returns:
            Tool execution result
        """
        if tool_name == "read":
            return self.read(
                file_path=tool_input.get("file_path", ""),
                offset=tool_input.get("offset", 0),
                limit=tool_input.get("limit")
            )
        elif tool_name == "edit":
            return self.edit(
                file_path=tool_input.get("file_path", ""),
                old_string=tool_input.get("old_string", ""),
                new_string=tool_input.get("new_string", ""),
                replace_all=tool_input.get("replace_all", False)
            )
        elif tool_name == "write":
            return self.write(
                file_path=tool_input.get("file_path", ""),
                content=tool_input.get("content", "")
            )
        elif tool_name == "bash":
            return self.bash(
                command=tool_input.get("command", ""),
                timeout=tool_input.get("timeout", 120)
            )
        elif tool_name == "grep":
            return self.grep(
                pattern=tool_input.get("pattern", ""),
                path=tool_input.get("path", "."),
                glob_pattern=tool_input.get("glob", "*"),
                case_insensitive=tool_input.get("case_insensitive", False),
                context_lines=tool_input.get("context_lines", 0)
            )
        elif tool_name == "glob":
            return self.glob(
                pattern=tool_input.get("pattern", "*"),
                path=tool_input.get("path", ".")
            )
        else:
            raise ToolExecutionError(f"Unknown tool: {tool_name}")


# Tool definitions for Claude API
TOOL_DEFINITIONS = [
    {
        "name": "read",
        "description": "Read a file from the workspace. Returns file contents with line numbers.",
        "input_schema": {
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to read (relative to workspace or absolute)"
                },
                "offset": {
                    "type": "integer",
                    "description": "Line number to start reading from (0-indexed). Optional.",
                    "default": 0
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of lines to read. Optional."
                }
            },
            "required": ["file_path"]
        }
    },
    {
        "name": "edit",
        "description": "Edit a file by replacing exact string matches. The old_string must match exactly.",
        "input_schema": {
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "Exact string to find and replace"
                },
                "new_string": {
                    "type": "string",
                    "description": "Replacement string"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "If true, replace all occurrences. If false, require unique match.",
                    "default": False
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        }
    },
    {
        "name": "write",
        "description": "Write content to a file (creates or overwrites). Creates parent directories if needed.",
        "input_schema": {
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["file_path", "content"]
        }
    },
    {
        "name": "bash",
        "description": "Execute a bash command in the workspace. Returns stdout, stderr, and exit code.",
        "input_schema": {
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "timeout": {
                    "type": "integer",
                    "description": "Maximum execution time in seconds",
                    "default": 120
                }
            },
            "required": ["command"]
        }
    },
    {
        "name": "grep",
        "description": "Search for a regex pattern in files. Returns matches with file, line number, and content.",
        "input_schema": {
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory or file to search in",
                    "default": "."
                },
                "glob": {
                    "type": "string",
                    "description": "File pattern to match (e.g., '*.py')",
                    "default": "*"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case-insensitive search",
                    "default": False
                },
                "context_lines": {
                    "type": "integer",
                    "description": "Number of context lines to show before/after match",
                    "default": 0
                }
            },
            "required": ["pattern"]
        }
    },
    {
        "name": "glob",
        "description": "Find files matching a glob pattern. Returns list of file paths.",
        "input_schema": {
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern (e.g., '**/*.py', 'src/*.js')"
                },
                "path": {
                    "type": "string",
                    "description": "Base directory to search in",
                    "default": "."
                }
            },
            "required": ["pattern"]
        }
    }
]
