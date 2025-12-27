/**
 * Shared Tools Module
 * Tools available to both Queen and Worker agents
 */

const config = require('../config');
const monitor = require('../monitor');

/**
 * Tool definitions for shared tools
 */
const TOOL_DEFINITIONS = [
  {
    name: 'hive_get_config',
    description: 'Read configuration from hive.yaml. Use dot-notation for nested values (e.g., "monitoring.queen.enabled").',
    inputSchema: {
      type: 'object',
      properties: {
        key: {
          type: 'string',
          description: 'Config key path in dot-notation. If omitted, returns the entire config.'
        }
      },
      required: []
    }
  },
  {
    name: 'hive_start_monitoring',
    description: 'Start background monitoring loop. Queen monitors all drones, Worker monitors for new tasks. Uses pub/sub for real-time events.',
    inputSchema: {
      type: 'object',
      properties: {},
      required: []
    }
  },
  {
    name: 'hive_stop_monitoring',
    description: 'Stop the background monitoring loop.',
    inputSchema: {
      type: 'object',
      properties: {},
      required: []
    }
  },
  {
    name: 'hive_get_monitoring_events',
    description: 'Get pending monitoring events that have been collected in the background. Returns and clears the event queue.',
    inputSchema: {
      type: 'object',
      properties: {
        limit: {
          type: 'number',
          description: 'Maximum number of events to return (default: 100)'
        }
      },
      required: []
    }
  }
];

/**
 * Tool handlers
 */
const handlers = {
  /**
   * Get configuration value(s)
   */
  async hive_get_config(args) {
    try {
      if (args.key) {
        const value = config.getConfigValue(args.key);
        if (value === null || value === undefined) {
          return { success: false, error: `Config key not found: ${args.key}` };
        }
        return { success: true, key: args.key, value };
      } else {
        const fullConfig = config.loadConfig();
        return { success: true, config: fullConfig };
      }
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Start background monitoring
   */
  async hive_start_monitoring() {
    try {
      const settings = config.getMonitoringSettings();
      if (!settings.enabled) {
        return {
          success: false,
          error: 'Monitoring is disabled in config. Set monitoring.<role>.enabled to true in hive.yaml'
        };
      }

      const isRunning = monitor.isRunning();
      if (isRunning) {
        return {
          success: true,
          message: 'Monitoring is already running',
          interval_seconds: settings.intervalSeconds
        };
      }

      await monitor.start();
      return {
        success: true,
        message: 'Monitoring started',
        role: config.isQueen() ? 'queen' : 'worker',
        interval_seconds: settings.intervalSeconds
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Stop background monitoring
   */
  async hive_stop_monitoring() {
    try {
      const wasRunning = monitor.isRunning();
      await monitor.stop();
      return {
        success: true,
        message: wasRunning ? 'Monitoring stopped' : 'Monitoring was not running'
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Get pending monitoring events
   */
  async hive_get_monitoring_events(args) {
    try {
      const limit = args.limit || 100;
      const events = monitor.getEvents(limit);
      return {
        success: true,
        count: events.length,
        monitoring_active: monitor.isRunning(),
        events
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  }
};

module.exports = {
  definitions: TOOL_DEFINITIONS,
  handlers
};
