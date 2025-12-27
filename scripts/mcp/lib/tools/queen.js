/**
 * Queen Tools Module
 * Tools for the orchestrator (Queen) agent
 */

const { getClient, Keys } = require('../redis-client');
const config = require('../config');

/**
 * Tool definitions for Queen
 */
const TOOL_DEFINITIONS = [
  {
    name: 'hive_status',
    description: 'Get complete HIVE status: all drones, their queued/active/completed/failed tasks, and overall system health.',
    inputSchema: {
      type: 'object',
      properties: {},
      required: []
    }
  },
  {
    name: 'hive_assign',
    description: 'Assign a task to a specific drone. The task will be queued if the drone is busy.',
    inputSchema: {
      type: 'object',
      properties: {
        drone: {
          type: 'string',
          description: 'Target drone name (e.g., "drone-1")'
        },
        title: {
          type: 'string',
          description: 'Short task title'
        },
        description: {
          type: 'string',
          description: 'Detailed task description with acceptance criteria'
        },
        ticket_id: {
          type: 'string',
          description: 'Optional ticket/issue ID (e.g., "PROJ-123")'
        },
        priority: {
          type: 'string',
          enum: ['low', 'normal', 'high'],
          description: 'Task priority (default: normal)'
        }
      },
      required: ['drone', 'title', 'description']
    }
  },
  {
    name: 'hive_submit',
    description: 'Submit a task to be auto-assigned to the least loaded drone.',
    inputSchema: {
      type: 'object',
      properties: {
        title: {
          type: 'string',
          description: 'Short task title'
        },
        description: {
          type: 'string',
          description: 'Detailed task description with acceptance criteria'
        },
        ticket_id: {
          type: 'string',
          description: 'Optional ticket/issue ID'
        },
        priority: {
          type: 'string',
          enum: ['low', 'normal', 'high'],
          description: 'Task priority (default: normal)'
        }
      },
      required: ['title', 'description']
    }
  },
  {
    name: 'hive_get_drone_activity',
    description: 'Get activity logs from a specific drone. Shows recent actions, tool calls, and responses.',
    inputSchema: {
      type: 'object',
      properties: {
        drone: {
          type: 'string',
          description: 'Drone name to get logs for (e.g., "drone-1")'
        },
        limit: {
          type: 'number',
          description: 'Number of log entries to retrieve (default: 50)'
        }
      },
      required: ['drone']
    }
  },
  {
    name: 'hive_get_failed_tasks',
    description: 'Get all failed tasks across all drones, with error details.',
    inputSchema: {
      type: 'object',
      properties: {
        drone: {
          type: 'string',
          description: 'Filter by drone name (optional, shows all if omitted)'
        }
      },
      required: []
    }
  },
  {
    name: 'hive_broadcast',
    description: 'Broadcast a message to all drones via pub/sub channel.',
    inputSchema: {
      type: 'object',
      properties: {
        message: {
          type: 'string',
          description: 'Message to broadcast'
        },
        type: {
          type: 'string',
          enum: ['info', 'warning', 'urgent'],
          description: 'Message type (default: info)'
        }
      },
      required: ['message']
    }
  }
];

/**
 * Tool handlers
 */
const handlers = {
  /**
   * Get complete HIVE status
   */
  async hive_status() {
    try {
      const redis = await getClient();
      const workers = config.getWorkerNames();
      const status = {
        timestamp: new Date().toISOString(),
        drones: {}
      };

      let totalQueued = 0;
      let totalActive = 0;
      let totalCompleted = 0;
      let totalFailed = 0;

      for (const drone of workers) {
        const [queueLen, activeLen, completedLen, failedLen] = await Promise.all([
          redis.llen(Keys.queue(drone)),
          redis.llen(Keys.active(drone)),
          redis.llen(Keys.completed(drone)),
          redis.llen(Keys.failed(drone))
        ]);

        // Get current active task if any
        let activeTask = null;
        if (activeLen > 0) {
          const taskJson = await redis.lindex(Keys.active(drone), 0);
          if (taskJson) {
            try {
              activeTask = JSON.parse(taskJson);
            } catch (e) {
              activeTask = { raw: taskJson };
            }
          }
        }

        status.drones[drone] = {
          queued: queueLen,
          active: activeLen,
          completed: completedLen,
          failed: failedLen,
          current_task: activeTask
        };

        totalQueued += queueLen;
        totalActive += activeLen;
        totalCompleted += completedLen;
        totalFailed += failedLen;
      }

      status.summary = {
        total_drones: workers.length,
        total_queued: totalQueued,
        total_active: totalActive,
        total_completed: totalCompleted,
        total_failed: totalFailed
      };

      return { success: true, status };
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Assign task to specific drone
   */
  async hive_assign(args) {
    try {
      const redis = await getClient();
      const { drone, title, description, ticket_id, priority } = args;

      const task = {
        id: `task-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
        title,
        description,
        ticket_id: ticket_id || null,
        priority: priority || 'normal',
        assigned_to: drone,
        assigned_by: 'queen',
        created_at: new Date().toISOString(),
        status: 'queued'
      };

      // Push to drone's queue
      await redis.rpush(Keys.queue(drone), JSON.stringify(task));

      // Publish event for real-time notification
      await redis.publish(Keys.channel.tasks, JSON.stringify({
        type: 'task_assigned',
        drone,
        task_id: task.id,
        title
      }));

      return {
        success: true,
        task_id: task.id,
        assigned_to: drone,
        message: `Task "${title}" assigned to ${drone}`
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Auto-submit task to least loaded drone
   */
  async hive_submit(args) {
    try {
      const redis = await getClient();
      const workers = config.getWorkerNames();
      const { title, description, ticket_id, priority } = args;

      // Find drone with smallest queue
      let minQueue = Infinity;
      let targetDrone = workers[0];

      for (const drone of workers) {
        const queueLen = await redis.llen(Keys.queue(drone));
        const activeLen = await redis.llen(Keys.active(drone));
        const load = queueLen + activeLen;

        if (load < minQueue) {
          minQueue = load;
          targetDrone = drone;
        }
      }

      // Use hive_assign handler
      return handlers.hive_assign({
        drone: targetDrone,
        title,
        description,
        ticket_id,
        priority
      });
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Get drone activity logs
   */
  async hive_get_drone_activity(args) {
    try {
      const redis = await getClient();
      const { drone, limit = 50 } = args;

      // Read from Redis stream
      const entries = await redis.xrevrange(Keys.logs(drone), '+', '-', 'COUNT', limit);

      const logs = entries.map(([id, fields]) => {
        const entry = { id, timestamp: id.split('-')[0] };
        for (let i = 0; i < fields.length; i += 2) {
          entry[fields[i]] = fields[i + 1];
        }
        return entry;
      });

      // Get current status
      const [activeTask, queueLen] = await Promise.all([
        redis.lindex(Keys.active(drone), 0),
        redis.llen(Keys.queue(drone))
      ]);

      let currentTask = null;
      if (activeTask) {
        try {
          currentTask = JSON.parse(activeTask);
        } catch (e) {
          currentTask = { raw: activeTask };
        }
      }

      return {
        success: true,
        drone,
        current_task: currentTask,
        queued_tasks: queueLen,
        logs,
        log_count: logs.length
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Get failed tasks
   */
  async hive_get_failed_tasks(args) {
    try {
      const redis = await getClient();
      const { drone } = args;
      const workers = drone ? [drone] : config.getWorkerNames();

      const failed = [];

      for (const w of workers) {
        const tasks = await redis.lrange(Keys.failed(w), 0, -1);
        for (const taskJson of tasks) {
          try {
            const task = JSON.parse(taskJson);
            task.drone = w;
            failed.push(task);
          } catch (e) {
            failed.push({ drone: w, raw: taskJson });
          }
        }
      }

      // Sort by timestamp descending
      failed.sort((a, b) => {
        const ta = a.failed_at || a.created_at || '';
        const tb = b.failed_at || b.created_at || '';
        return tb.localeCompare(ta);
      });

      return {
        success: true,
        count: failed.length,
        failed_tasks: failed
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Broadcast message to all drones
   */
  async hive_broadcast(args) {
    try {
      const redis = await getClient();
      const { message, type = 'info' } = args;

      const payload = JSON.stringify({
        type: 'broadcast',
        message_type: type,
        message,
        from: 'queen',
        timestamp: new Date().toISOString()
      });

      const subscribers = await redis.publish(Keys.channel.broadcast, payload);

      return {
        success: true,
        message: 'Broadcast sent',
        subscribers_reached: subscribers
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
