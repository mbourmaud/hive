/**
 * Monitor Module
 * Background monitoring using Redis pub/sub and polling
 */

const { getClient, getSubscriber, Keys } = require('./redis-client');
const config = require('./config');

// Monitoring state
let running = false;
let pollInterval = null;
let subscriberClient = null;
const eventQueue = [];
const MAX_EVENTS = 1000;

/**
 * Add event to queue (FIFO with max size)
 */
function addEvent(event) {
  event.received_at = new Date().toISOString();
  eventQueue.push(event);

  // Keep queue bounded
  while (eventQueue.length > MAX_EVENTS) {
    eventQueue.shift();
  }
}

/**
 * Get and clear events from queue
 */
function getEvents(limit = 100) {
  const events = eventQueue.splice(0, Math.min(limit, eventQueue.length));
  return events;
}

/**
 * Check if monitoring is running
 */
function isRunning() {
  return running;
}

/**
 * Start monitoring
 */
async function start() {
  if (running) {
    return;
  }

  const settings = config.getMonitoringSettings();
  if (!settings.enabled) {
    throw new Error('Monitoring is disabled in config');
  }

  running = true;
  const isQueenAgent = config.isQueen();
  const agentName = config.getAgentName();

  // Subscribe to relevant channels
  try {
    subscriberClient = await getSubscriber();

    // All agents subscribe to broadcast
    await subscriberClient.subscribe(Keys.channel.broadcast);

    // Queen subscribes to events and tasks
    if (isQueenAgent) {
      await subscriberClient.subscribe(Keys.channel.events);
      await subscriberClient.subscribe(Keys.channel.tasks);
    } else {
      // Workers subscribe to tasks channel for assignments
      await subscriberClient.subscribe(Keys.channel.tasks);
    }

    subscriberClient.on('message', (channel, message) => {
      try {
        const event = JSON.parse(message);
        event.channel = channel;

        // Filter events for workers - only care about their own assignments
        if (!isQueenAgent && channel === Keys.channel.tasks) {
          if (event.drone && event.drone !== agentName) {
            return; // Ignore tasks for other drones
          }
        }

        addEvent(event);
      } catch (e) {
        addEvent({ channel, raw: message, parse_error: e.message });
      }
    });
  } catch (error) {
    console.error('[monitor] Failed to setup pub/sub:', error.message);
  }

  // Start polling loop
  const intervalMs = settings.intervalSeconds * 1000;

  if (isQueenAgent) {
    // Queen: Poll for status changes
    pollInterval = setInterval(async () => {
      try {
        await pollQueenStatus();
      } catch (error) {
        addEvent({ type: 'poll_error', error: error.message });
      }
    }, intervalMs);
  } else {
    // Worker: Poll for new tasks
    pollInterval = setInterval(async () => {
      try {
        await pollWorkerTasks();
      } catch (error) {
        addEvent({ type: 'poll_error', error: error.message });
      }
    }, intervalMs);
  }
}

/**
 * Stop monitoring
 */
async function stop() {
  if (!running) {
    return;
  }

  running = false;

  if (pollInterval) {
    clearInterval(pollInterval);
    pollInterval = null;
  }

  if (subscriberClient) {
    try {
      await subscriberClient.unsubscribe();
    } catch (e) {
      // Ignore unsubscribe errors
    }
    subscriberClient = null;
  }
}

/**
 * Queen polling: Check all drone statuses
 */
async function pollQueenStatus() {
  const redis = await getClient();
  const workers = config.getWorkerNames();

  for (const drone of workers) {
    // Check for new completed tasks
    const completedLen = await redis.llen(Keys.completed(drone));
    const failedLen = await redis.llen(Keys.failed(drone));

    // Get latest log entry
    const logs = await redis.xrevrange(Keys.logs(drone), '+', '-', 'COUNT', 1);

    if (logs.length > 0) {
      const [id, fields] = logs[0];
      const entry = { id };
      for (let i = 0; i < fields.length; i += 2) {
        entry[fields[i]] = fields[i + 1];
      }

      // Add summary event
      addEvent({
        type: 'drone_status',
        drone,
        completed_count: completedLen,
        failed_count: failedLen,
        latest_activity: entry
      });
    }
  }
}

/**
 * Worker polling: Check for new tasks in queue
 */
async function pollWorkerTasks() {
  const redis = await getClient();
  const drone = config.getAgentName();

  const queueLen = await redis.llen(Keys.queue(drone));
  const activeLen = await redis.llen(Keys.active(drone));

  if (queueLen > 0 && activeLen === 0) {
    // New task available and not busy
    const taskJson = await redis.lindex(Keys.queue(drone), 0);
    let task = null;
    try {
      task = JSON.parse(taskJson);
    } catch (e) {
      task = { raw: taskJson };
    }

    addEvent({
      type: 'new_task_available',
      drone,
      queued: queueLen,
      next_task: task
    });
  }
}

module.exports = {
  start,
  stop,
  isRunning,
  getEvents,
  addEvent
};
