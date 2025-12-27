/**
 * Config Module
 * Read and parse hive.yaml configuration
 */

const fs = require('fs');
const yaml = require('js-yaml');
const path = require('path');

// Config file paths (in order of priority)
const CONFIG_PATHS = [
  '/hive-config/hive.yaml',
  '/workspace/hive.yaml',
  process.env.HIVE_CONFIG_PATH
].filter(Boolean);

let cachedConfig = null;
let cachedAt = 0;
const CACHE_TTL = 30000; // 30 seconds

/**
 * Find and load the hive.yaml config file
 */
function loadConfig() {
  const now = Date.now();
  if (cachedConfig && (now - cachedAt) < CACHE_TTL) {
    return cachedConfig;
  }

  for (const configPath of CONFIG_PATHS) {
    try {
      if (fs.existsSync(configPath)) {
        const content = fs.readFileSync(configPath, 'utf8');
        cachedConfig = yaml.load(content) || {};
        cachedAt = now;
        return cachedConfig;
      }
    } catch (err) {
      // Continue to next path
    }
  }

  // Return defaults if no config found
  return getDefaultConfig();
}

/**
 * Default configuration
 */
function getDefaultConfig() {
  return {
    workspace: {
      name: process.env.WORKSPACE_NAME || 'workspace'
    },
    agents: {
      queen: { name: 'queen' },
      workers: [{ name: 'drone-1' }, { name: 'drone-2' }]
    },
    monitoring: {
      queen: {
        enabled: true,
        interval_seconds: 30
      },
      worker: {
        enabled: true,
        interval_seconds: 1
      }
    },
    redis: {
      host: process.env.REDIS_HOST || 'redis',
      port: parseInt(process.env.REDIS_PORT || '6379', 10),
      password: process.env.REDIS_PASSWORD || 'hiveredis'
    }
  };
}

/**
 * Get a nested config value by dot-notation path
 * Example: getConfigValue('monitoring.queen.enabled')
 */
function getConfigValue(keyPath, defaultValue = null) {
  const config = loadConfig();
  const keys = keyPath.split('.');
  let value = config;

  for (const key of keys) {
    if (value === undefined || value === null) {
      return defaultValue;
    }
    value = value[key];
  }

  return value !== undefined ? value : defaultValue;
}

/**
 * Get the current agent's role from environment
 */
function getAgentRole() {
  return process.env.AGENT_ROLE || 'worker';
}

/**
 * Get the current agent's name from environment
 */
function getAgentName() {
  return process.env.AGENT_NAME || process.env.HOSTNAME || 'unknown';
}

/**
 * Check if agent is the queen
 */
function isQueen() {
  return getAgentRole() === 'orchestrator' || getAgentName() === 'queen';
}

/**
 * Get list of all worker names from config
 */
function getWorkerNames() {
  const cfg = loadConfig();
  const workers = cfg.agents?.workers;

  // Handle array format: [{name: "drone-1"}, {name: "drone-2"}]
  if (Array.isArray(workers)) {
    return workers.map((w, i) => w.name || `drone-${i + 1}`);
  }

  // Handle count format: {count: 2, ...}
  if (workers && typeof workers.count === 'number') {
    const names = [];
    for (let i = 1; i <= workers.count; i++) {
      names.push(`drone-${i}`);
    }
    return names;
  }

  // Fallback: detect from environment or default
  const workerCount = parseInt(process.env.WORKER_COUNT || '2', 10);
  const names = [];
  for (let i = 1; i <= workerCount; i++) {
    names.push(`drone-${i}`);
  }
  return names;
}

/**
 * Get monitoring settings for current role
 */
function getMonitoringSettings() {
  const role = isQueen() ? 'queen' : 'worker';
  return {
    enabled: getConfigValue(`monitoring.${role}.enabled`, true),
    intervalSeconds: getConfigValue(`monitoring.${role}.interval_seconds`, role === 'queen' ? 30 : 1)
  };
}

/**
 * Invalidate config cache (force reload on next access)
 */
function invalidateCache() {
  cachedConfig = null;
  cachedAt = 0;
}

module.exports = {
  loadConfig,
  getConfigValue,
  getAgentRole,
  getAgentName,
  isQueen,
  getWorkerNames,
  getMonitoringSettings,
  invalidateCache
};
