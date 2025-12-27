/**
 * Worker Tools Module
 * Tools for drone (Worker) agents
 */

const { getClient, Keys } = require('../redis-client');
const config = require('../config');

/**
 * Tool definitions for Worker
 */
const TOOL_DEFINITIONS = [
  {
    name: 'hive_my_tasks',
    description: 'Get your current active task and queued tasks. Shows what you are working on and what is coming next.',
    inputSchema: {
      type: 'object',
      properties: {},
      required: []
    }
  },
  {
    name: 'hive_take_task',
    description: 'Take the next task from your queue and start working on it. Moves task from queued to active.',
    inputSchema: {
      type: 'object',
      properties: {},
      required: []
    }
  },
  {
    name: 'hive_complete_task',
    description: 'Mark your current task as completed. Moves from active to completed with result.',
    inputSchema: {
      type: 'object',
      properties: {
        result: {
          type: 'string',
          description: 'Completion summary (e.g., "PR created: #123, CI green")'
        }
      },
      required: ['result']
    }
  },
  {
    name: 'hive_fail_task',
    description: 'Mark your current task as failed. Use when you cannot complete the task.',
    inputSchema: {
      type: 'object',
      properties: {
        error: {
          type: 'string',
          description: 'Error description explaining why the task failed'
        }
      },
      required: ['error']
    }
  },
  {
    name: 'hive_log_activity',
    description: 'Log activity for Queen visibility. Use frequently to keep Queen informed of progress.',
    inputSchema: {
      type: 'object',
      properties: {
        message: {
          type: 'string',
          description: 'Activity message (use emoji prefixes: 📋 task, 🔧 tool, 📖 read, ✏️ edit, 🚀 server, ✅ done, 🚫 blocked)'
        },
        level: {
          type: 'string',
          enum: ['debug', 'info', 'warning', 'error'],
          description: 'Log level (default: info)'
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
   * Get my tasks (active + queued)
   */
  async hive_my_tasks() {
    try {
      const redis = await getClient();
      const drone = config.getAgentName();

      const [activeList, queuedList] = await Promise.all([
        redis.lrange(Keys.active(drone), 0, -1),
        redis.lrange(Keys.queue(drone), 0, -1)
      ]);

      const active = activeList.map(json => {
        try {
          return JSON.parse(json);
        } catch (e) {
          return { raw: json };
        }
      });

      const queued = queuedList.map(json => {
        try {
          return JSON.parse(json);
        } catch (e) {
          return { raw: json };
        }
      });

      return {
        success: true,
        drone,
        active_task: active.length > 0 ? active[0] : null,
        queued_count: queued.length,
        queued_tasks: queued,
        status: active.length > 0 ? 'WORKING' : (queued.length > 0 ? 'HAS_QUEUED' : 'IDLE')
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Take next task from queue
   */
  async hive_take_task() {
    try {
      const redis = await getClient();
      const drone = config.getAgentName();

      // Check if already has active task
      const activeCount = await redis.llen(Keys.active(drone));
      if (activeCount > 0) {
        const currentTask = await redis.lindex(Keys.active(drone), 0);
        let task = null;
        try {
          task = JSON.parse(currentTask);
        } catch (e) {
          task = { raw: currentTask };
        }
        return {
          success: false,
          error: 'Already have an active task. Complete or fail it first.',
          current_task: task
        };
      }

      // Pop from queue
      const taskJson = await redis.lpop(Keys.queue(drone));
      if (!taskJson) {
        return {
          success: false,
          error: 'No tasks in queue',
          status: 'IDLE'
        };
      }

      let task;
      try {
        task = JSON.parse(taskJson);
      } catch (e) {
        task = { raw: taskJson, id: `task-${Date.now()}` };
      }

      // Update task status and add to active
      task.status = 'in_progress';
      task.started_at = new Date().toISOString();

      await redis.rpush(Keys.active(drone), JSON.stringify(task));

      // Log the activity
      await redis.xadd(
        Keys.logs(drone),
        '*',
        'type', 'task_started',
        'level', 'info',
        'message', `Started task: ${task.title || task.id}`,
        'task_id', task.id
      );

      // Publish event
      await redis.publish(Keys.channel.events, JSON.stringify({
        type: 'task_started',
        drone,
        task_id: task.id,
        title: task.title
      }));

      return {
        success: true,
        message: 'Task taken',
        task
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Mark current task as completed
   */
  async hive_complete_task(args) {
    try {
      const redis = await getClient();
      const drone = config.getAgentName();
      const { result } = args;

      // Get active task
      const taskJson = await redis.lpop(Keys.active(drone));
      if (!taskJson) {
        return {
          success: false,
          error: 'No active task to complete'
        };
      }

      let task;
      try {
        task = JSON.parse(taskJson);
      } catch (e) {
        task = { raw: taskJson };
      }

      // Update task
      task.status = 'completed';
      task.completed_at = new Date().toISOString();
      task.result = result;

      // Add to completed list
      await redis.rpush(Keys.completed(drone), JSON.stringify(task));

      // Log the activity
      await redis.xadd(
        Keys.logs(drone),
        '*',
        'type', 'task_completed',
        'level', 'info',
        'message', `Completed task: ${task.title || task.id}`,
        'task_id', task.id,
        'result', result
      );

      // Publish event
      await redis.publish(Keys.channel.events, JSON.stringify({
        type: 'task_completed',
        drone,
        task_id: task.id,
        title: task.title,
        result
      }));

      // Check for next task
      const queueLen = await redis.llen(Keys.queue(drone));

      return {
        success: true,
        message: 'Task completed',
        task_id: task.id,
        queued_tasks: queueLen,
        next_action: queueLen > 0 ? 'Use hive_take_task to start next task' : 'No more tasks in queue'
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Mark current task as failed
   */
  async hive_fail_task(args) {
    try {
      const redis = await getClient();
      const drone = config.getAgentName();
      const { error: errorMsg } = args;

      // Get active task
      const taskJson = await redis.lpop(Keys.active(drone));
      if (!taskJson) {
        return {
          success: false,
          error: 'No active task to fail'
        };
      }

      let task;
      try {
        task = JSON.parse(taskJson);
      } catch (e) {
        task = { raw: taskJson };
      }

      // Update task
      task.status = 'failed';
      task.failed_at = new Date().toISOString();
      task.error = errorMsg;

      // Add to failed list
      await redis.rpush(Keys.failed(drone), JSON.stringify(task));

      // Log the activity
      await redis.xadd(
        Keys.logs(drone),
        '*',
        'type', 'task_failed',
        'level', 'error',
        'message', `Failed task: ${task.title || task.id} - ${errorMsg}`,
        'task_id', task.id,
        'error', errorMsg
      );

      // Publish event
      await redis.publish(Keys.channel.events, JSON.stringify({
        type: 'task_failed',
        drone,
        task_id: task.id,
        title: task.title,
        error: errorMsg
      }));

      // Check for next task
      const queueLen = await redis.llen(Keys.queue(drone));

      return {
        success: true,
        message: 'Task marked as failed',
        task_id: task.id,
        queued_tasks: queueLen,
        next_action: queueLen > 0 ? 'Use hive_take_task to start next task' : 'No more tasks in queue'
      };
    } catch (error) {
      return { success: false, error: error.message };
    }
  },

  /**
   * Log activity for Queen visibility
   */
  async hive_log_activity(args) {
    try {
      const redis = await getClient();
      const drone = config.getAgentName();
      const { message, level = 'info' } = args;

      // Add to Redis stream
      const entryId = await redis.xadd(
        Keys.logs(drone),
        '*',
        'type', 'activity',
        'level', level,
        'message', message,
        'timestamp', new Date().toISOString()
      );

      // Publish for real-time subscribers
      await redis.publish(Keys.channel.events, JSON.stringify({
        type: 'activity_log',
        drone,
        level,
        message,
        entry_id: entryId
      }));

      return {
        success: true,
        entry_id: entryId,
        message: 'Activity logged'
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
