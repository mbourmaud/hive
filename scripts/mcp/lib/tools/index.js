/**
 * Tools Registry
 * Aggregates and exposes tools based on agent role
 */

const config = require('../config');
const sharedTools = require('./shared');
const queenTools = require('./queen');
const workerTools = require('./worker');

/**
 * Get all tool definitions for the current agent
 */
function getToolDefinitions() {
  const isQueenAgent = config.isQueen();

  // Always include shared tools
  const definitions = [...sharedTools.definitions];

  // Add role-specific tools
  if (isQueenAgent) {
    definitions.push(...queenTools.definitions);
  } else {
    definitions.push(...workerTools.definitions);
  }

  return definitions;
}

/**
 * Get the handler for a specific tool
 */
function getHandler(toolName) {
  // Check shared tools first
  if (sharedTools.handlers[toolName]) {
    return sharedTools.handlers[toolName];
  }

  // Check queen tools
  if (queenTools.handlers[toolName]) {
    return queenTools.handlers[toolName];
  }

  // Check worker tools
  if (workerTools.handlers[toolName]) {
    return workerTools.handlers[toolName];
  }

  return null;
}

/**
 * Execute a tool by name with given arguments
 */
async function executeTool(toolName, args = {}) {
  const handler = getHandler(toolName);

  if (!handler) {
    return {
      success: false,
      error: `Unknown tool: ${toolName}`
    };
  }

  try {
    return await handler(args);
  } catch (error) {
    return {
      success: false,
      error: `Tool execution error: ${error.message}`
    };
  }
}

/**
 * Check if a tool is available for the current agent
 */
function isToolAvailable(toolName) {
  const definitions = getToolDefinitions();
  return definitions.some(def => def.name === toolName);
}

module.exports = {
  getToolDefinitions,
  getHandler,
  executeTool,
  isToolAvailable
};
