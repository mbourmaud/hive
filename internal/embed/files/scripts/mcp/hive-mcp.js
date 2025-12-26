#!/usr/bin/env node
/**
 * Hive MCP Server
 * Exposes Hive commands as MCP tools for elegant Claude integration
 *
 * Tools:
 *   - hive_status: Get task queue and drone status
 *   - hive_submit: Submit a task to available drone
 *   - hive_log: View drone activity logs
 */

const { execSync } = require('child_process');
const readline = require('readline');

// Tool definitions
const TOOLS = [
  {
    name: 'hive_status',
    description: 'Get the current status of the Hive task queue and drone workers. Shows queued, in-progress, and completed tasks.',
    inputSchema: {
      type: 'object',
      properties: {},
      required: []
    }
  },
  {
    name: 'hive_submit',
    description: 'Submit a new task to be picked up by an available drone worker. The task will be queued and assigned to the next free drone.',
    inputSchema: {
      type: 'object',
      properties: {
        task: {
          type: 'string',
          description: 'The task description to submit. Be specific: include file paths, function names, and acceptance criteria.'
        },
        priority: {
          type: 'string',
          enum: ['low', 'normal', 'high'],
          description: 'Task priority (default: normal)'
        }
      },
      required: ['task']
    }
  },
  {
    name: 'hive_log',
    description: 'View activity logs from drone workers. Shows recent actions and outputs from drones.',
    inputSchema: {
      type: 'object',
      properties: {
        drone: {
          type: 'string',
          description: 'Drone ID to view logs for (e.g., "drone-1"). If omitted, shows logs from all drones.'
        },
        lines: {
          type: 'number',
          description: 'Number of log lines to show (default: 50)'
        }
      },
      required: []
    }
  }
];

// Execute a command and return output
function runCommand(cmd) {
  try {
    return execSync(cmd, { encoding: 'utf-8', timeout: 30000 }).trim();
  } catch (error) {
    return `Error: ${error.message}`;
  }
}

// Handle tool calls
function handleToolCall(name, args) {
  switch (name) {
    case 'hive_status':
      return runCommand('hive-status');

    case 'hive_submit':
      const task = args.task;
      const priority = args.priority || 'normal';
      // Escape quotes in task
      const escapedTask = task.replace(/"/g, '\\"');
      return runCommand(`hive-submit "${escapedTask}"`);

    case 'hive_log':
      const drone = args.drone || '';
      const lines = args.lines || 50;
      if (drone) {
        return runCommand(`hive-log ${drone} ${lines}`);
      }
      return runCommand(`hive-log ${lines}`);

    default:
      return `Unknown tool: ${name}`;
  }
}

// JSON-RPC response helpers
function jsonRpcResponse(id, result) {
  return JSON.stringify({ jsonrpc: '2.0', id, result });
}

function jsonRpcError(id, code, message) {
  return JSON.stringify({ jsonrpc: '2.0', id, error: { code, message } });
}

// Main MCP server loop
async function main() {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false
  });

  for await (const line of rl) {
    try {
      const request = JSON.parse(line);
      const { id, method, params } = request;

      let response;
      switch (method) {
        case 'initialize':
          response = jsonRpcResponse(id, {
            protocolVersion: '2024-11-05',
            capabilities: { tools: {} },
            serverInfo: { name: 'hive-mcp', version: '1.0.0' }
          });
          break;

        case 'tools/list':
          response = jsonRpcResponse(id, { tools: TOOLS });
          break;

        case 'tools/call':
          const { name, arguments: args } = params;
          const result = handleToolCall(name, args || {});
          response = jsonRpcResponse(id, {
            content: [{ type: 'text', text: result }]
          });
          break;

        case 'notifications/initialized':
          // No response needed for notifications
          continue;

        default:
          response = jsonRpcError(id, -32601, `Method not found: ${method}`);
      }

      if (response) {
        console.log(response);
      }
    } catch (error) {
      console.error(JSON.stringify({
        jsonrpc: '2.0',
        id: null,
        error: { code: -32700, message: `Parse error: ${error.message}` }
      }));
    }
  }
}

main().catch(console.error);
