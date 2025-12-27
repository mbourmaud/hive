#!/usr/bin/env node
/**
 * HIVE MCP Server v2.0
 *
 * Native Redis connection using ioredis (no shell commands, no timeouts)
 * Role-aware tools: Queen gets orchestration tools, Workers get execution tools
 * Background monitoring with pub/sub
 * Config access from hive.yaml
 *
 * Queen Tools:
 *   - hive_status: Get complete HIVE status
 *   - hive_assign: Assign task to specific drone
 *   - hive_submit: Auto-assign to least loaded drone
 *   - hive_get_drone_activity: Get drone logs
 *   - hive_get_failed_tasks: List failed tasks
 *   - hive_broadcast: Message all drones
 *
 * Worker Tools:
 *   - hive_my_tasks: Get active/queued tasks
 *   - hive_take_task: Take next task from queue
 *   - hive_complete_task: Mark task done
 *   - hive_fail_task: Mark task failed
 *   - hive_log_activity: Log activity for Queen
 *
 * Shared Tools:
 *   - hive_get_config: Read hive.yaml config
 *   - hive_start_monitoring: Start background monitoring
 *   - hive_stop_monitoring: Stop monitoring
 *   - hive_get_monitoring_events: Get pending events
 */

const readline = require('readline');
const tools = require('./lib/tools');
const config = require('./lib/config');
const { closeAll } = require('./lib/redis-client');

// JSON-RPC response helpers
function jsonRpcResponse(id, result) {
  return JSON.stringify({ jsonrpc: '2.0', id, result });
}

function jsonRpcError(id, code, message) {
  return JSON.stringify({ jsonrpc: '2.0', id, error: { code, message } });
}

// Handle tool calls
async function handleToolCall(name, args) {
  const result = await tools.executeTool(name, args || {});

  // Format as text for MCP response
  if (typeof result === 'object') {
    return JSON.stringify(result, null, 2);
  }
  return String(result);
}

// Main MCP server loop
async function main() {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
    terminal: false
  });

  // Get role info for server info
  const agentName = config.getAgentName();
  const isQueenAgent = config.isQueen();
  const role = isQueenAgent ? 'queen' : 'worker';

  // Handle graceful shutdown
  process.on('SIGINT', async () => {
    await closeAll();
    process.exit(0);
  });

  process.on('SIGTERM', async () => {
    await closeAll();
    process.exit(0);
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
            serverInfo: {
              name: 'hive-mcp',
              version: '2.0.0',
              description: `HIVE MCP Server (${role}: ${agentName})`
            }
          });
          break;

        case 'tools/list':
          const toolDefinitions = tools.getToolDefinitions();
          response = jsonRpcResponse(id, { tools: toolDefinitions });
          break;

        case 'tools/call':
          const { name, arguments: args } = params;
          try {
            const result = await handleToolCall(name, args);
            response = jsonRpcResponse(id, {
              content: [{ type: 'text', text: result }]
            });
          } catch (error) {
            response = jsonRpcResponse(id, {
              content: [{ type: 'text', text: `Error: ${error.message}` }],
              isError: true
            });
          }
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

  // Cleanup on stream end
  await closeAll();
}

main().catch(async (error) => {
  console.error('Fatal error:', error);
  await closeAll();
  process.exit(1);
});
