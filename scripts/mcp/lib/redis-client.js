/**
 * Redis Client Module
 * Native Redis connection using ioredis
 */

const Redis = require('ioredis');

let client = null;
let subscriber = null;

/**
 * Get Redis connection options from environment
 */
function getRedisOptions() {
  return {
    host: process.env.REDIS_HOST || 'redis',
    port: parseInt(process.env.REDIS_PORT || '6379', 10),
    password: process.env.REDIS_PASSWORD || 'hiveredis',
    retryStrategy: (times) => {
      if (times > 3) return null;
      return Math.min(times * 100, 1000);
    },
    lazyConnect: true,
    maxRetriesPerRequest: 3,
    connectTimeout: 5000
  };
}

/**
 * Get or create the Redis client singleton
 */
async function getClient() {
  if (!client) {
    client = new Redis(getRedisOptions());

    client.on('error', (err) => {
      console.error('[hive-mcp] Redis error:', err.message);
    });

    client.on('connect', () => {
      // Connected
    });

    await client.connect();
  }
  return client;
}

/**
 * Get or create a subscriber client for pub/sub
 */
async function getSubscriber() {
  if (!subscriber) {
    subscriber = new Redis(getRedisOptions());

    subscriber.on('error', (err) => {
      console.error('[hive-mcp] Redis subscriber error:', err.message);
    });

    await subscriber.connect();
  }
  return subscriber;
}

/**
 * Close all Redis connections
 */
async function closeAll() {
  if (client) {
    await client.quit();
    client = null;
  }
  if (subscriber) {
    await subscriber.quit();
    subscriber = null;
  }
}

/**
 * Key helpers for HIVE data structures
 */
const Keys = {
  // Task queues
  queue: (drone) => `hive:queue:${drone}`,
  active: (drone) => `hive:active:${drone}`,
  completed: (drone) => `hive:completed:${drone}`,
  failed: (drone) => `hive:failed:${drone}`,

  // Activity logs (Redis Streams)
  logs: (drone) => `hive:logs:${drone}`,

  // Pub/sub channels
  channel: {
    tasks: 'hive:channel:tasks',
    broadcast: 'hive:channel:broadcast',
    events: 'hive:channel:events'
  },

  // Agent patterns
  allQueues: 'hive:queue:*',
  allActive: 'hive:active:*',
  allLogs: 'hive:logs:*'
};

module.exports = {
  getClient,
  getSubscriber,
  closeAll,
  Keys
};
