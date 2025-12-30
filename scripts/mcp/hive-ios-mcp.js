#!/usr/bin/env node
/**
 * HIVE iOS MCP Server
 *
 * Provides iOS Simulator automation via xcrun simctl
 * Runs on macOS host and exposes tools via SSE transport
 *
 * Tools:
 *   - ios_list_devices: List available simulators
 *   - ios_boot_device: Boot a simulator
 *   - ios_shutdown_device: Shutdown a simulator
 *   - ios_install_app: Install .app bundle
 *   - ios_launch_app: Launch app by bundle ID
 *   - ios_terminate_app: Stop running app
 *   - ios_list_apps: List installed apps
 *   - ios_screenshot: Take screenshot
 *   - ios_open_url: Open URL in Safari
 *   - ios_set_location: Set GPS location
 *   - ios_push_notification: Send push notification
 *   - ios_get_status: Get current simulator status
 */

const http = require('http');
const { spawn, execSync } = require('child_process');
const { URL } = require('url');
const fs = require('fs');
const path = require('path');
const os = require('os');

// Parse command line arguments
const args = process.argv.slice(2);
let port = 8932;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--port' && args[i + 1]) {
    port = parseInt(args[i + 1], 10);
  }
}

// Tool definitions
const TOOLS = [
  {
    name: 'ios_list_devices',
    description: 'List all available iOS simulators with their states (Booted, Shutdown, etc.)',
    inputSchema: {
      type: 'object',
      properties: {
        filter: {
          type: 'string',
          enum: ['all', 'booted', 'available'],
          description: 'Filter devices: all (default), booted (running only), available (installed only)'
        }
      }
    }
  },
  {
    name: 'ios_boot_device',
    description: 'Boot (start) an iOS simulator by UDID or name',
    inputSchema: {
      type: 'object',
      properties: {
        device: {
          type: 'string',
          description: 'Device UDID or name (e.g., "iPhone 15" or "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX")'
        }
      },
      required: ['device']
    }
  },
  {
    name: 'ios_shutdown_device',
    description: 'Shutdown (stop) a running iOS simulator',
    inputSchema: {
      type: 'object',
      properties: {
        device: {
          type: 'string',
          description: 'Device UDID, name, or "booted" for the currently booted device'
        }
      },
      required: ['device']
    }
  },
  {
    name: 'ios_install_app',
    description: 'Install an .app bundle to the simulator',
    inputSchema: {
      type: 'object',
      properties: {
        device: {
          type: 'string',
          description: 'Device UDID, name, or "booted"'
        },
        appPath: {
          type: 'string',
          description: 'Path to the .app bundle to install'
        }
      },
      required: ['device', 'appPath']
    }
  },
  {
    name: 'ios_launch_app',
    description: 'Launch an installed app by its bundle identifier',
    inputSchema: {
      type: 'object',
      properties: {
        device: {
          type: 'string',
          description: 'Device UDID, name, or "booted"'
        },
        bundleId: {
          type: 'string',
          description: 'App bundle identifier (e.g., "com.apple.mobilesafari")'
        },
        args: {
          type: 'array',
          items: { type: 'string' },
          description: 'Optional launch arguments'
        }
      },
      required: ['device', 'bundleId']
    }
  },
  {
    name: 'ios_terminate_app',
    description: 'Terminate (stop) a running app',
    inputSchema: {
      type: 'object',
      properties: {
        device: {
          type: 'string',
          description: 'Device UDID, name, or "booted"'
        },
        bundleId: {
          type: 'string',
          description: 'App bundle identifier to terminate'
        }
      },
      required: ['device', 'bundleId']
    }
  },
  {
    name: 'ios_list_apps',
    description: 'List installed apps on a simulator',
    inputSchema: {
      type: 'object',
      properties: {
        device: {
          type: 'string',
          description: 'Device UDID, name, or "booted"'
        }
      },
      required: ['device']
    }
  },
  {
    name: 'ios_screenshot',
    description: 'Take a screenshot of the simulator and save it to a file',
    inputSchema: {
      type: 'object',
      properties: {
        device: {
          type: 'string',
          description: 'Device UDID, name, or "booted"'
        },
        outputPath: {
          type: 'string',
          description: 'Path to save the screenshot (PNG format). If not provided, saves to temp directory.'
        }
      },
      required: ['device']
    }
  },
  {
    name: 'ios_open_url',
    description: 'Open a URL in the simulator (launches Safari or appropriate app)',
    inputSchema: {
      type: 'object',
      properties: {
        device: {
          type: 'string',
          description: 'Device UDID, name, or "booted"'
        },
        url: {
          type: 'string',
          description: 'URL to open (e.g., "https://example.com" or "myapp://deeplink")'
        }
      },
      required: ['device', 'url']
    }
  },
  {
    name: 'ios_set_location',
    description: 'Set the GPS location of the simulator',
    inputSchema: {
      type: 'object',
      properties: {
        device: {
          type: 'string',
          description: 'Device UDID, name, or "booted"'
        },
        latitude: {
          type: 'number',
          description: 'Latitude coordinate'
        },
        longitude: {
          type: 'number',
          description: 'Longitude coordinate'
        }
      },
      required: ['device', 'latitude', 'longitude']
    }
  },
  {
    name: 'ios_push_notification',
    description: 'Send a push notification to an app in the simulator',
    inputSchema: {
      type: 'object',
      properties: {
        device: {
          type: 'string',
          description: 'Device UDID, name, or "booted"'
        },
        bundleId: {
          type: 'string',
          description: 'Target app bundle identifier'
        },
        payload: {
          type: 'object',
          description: 'Push notification payload (APNS format)',
          properties: {
            aps: {
              type: 'object',
              properties: {
                alert: {
                  oneOf: [
                    { type: 'string' },
                    {
                      type: 'object',
                      properties: {
                        title: { type: 'string' },
                        body: { type: 'string' }
                      }
                    }
                  ]
                },
                badge: { type: 'number' },
                sound: { type: 'string' }
              }
            }
          }
        }
      },
      required: ['device', 'bundleId', 'payload']
    }
  },
  {
    name: 'ios_get_status',
    description: 'Get the current status of iOS simulators and Xcode',
    inputSchema: {
      type: 'object',
      properties: {}
    }
  }
];

// Execute xcrun simctl command
function simctl(...args) {
  try {
    const result = execSync(`xcrun simctl ${args.join(' ')}`, {
      encoding: 'utf8',
      maxBuffer: 10 * 1024 * 1024
    });
    return { success: true, output: result.trim() };
  } catch (error) {
    return { success: false, error: error.message, stderr: error.stderr?.toString() };
  }
}

// Execute simctl with JSON output
function simctlJson(...args) {
  const result = simctl(...args, '-j');
  if (result.success) {
    try {
      return { success: true, data: JSON.parse(result.output) };
    } catch (e) {
      return { success: false, error: 'Failed to parse JSON output' };
    }
  }
  return result;
}

// Tool implementations
const toolHandlers = {
  ios_list_devices: async (args) => {
    const filter = args.filter || 'all';
    const result = simctlJson('list', 'devices');

    if (!result.success) {
      return { error: result.error };
    }

    const devices = [];
    for (const [runtime, runtimeDevices] of Object.entries(result.data.devices || {})) {
      for (const device of runtimeDevices) {
        if (filter === 'booted' && device.state !== 'Booted') continue;
        if (filter === 'available' && !device.isAvailable) continue;

        devices.push({
          name: device.name,
          udid: device.udid,
          state: device.state,
          runtime: runtime.replace('com.apple.CoreSimulator.SimRuntime.', ''),
          isAvailable: device.isAvailable
        });
      }
    }

    return { devices, count: devices.length };
  },

  ios_boot_device: async (args) => {
    const result = simctl('boot', `"${args.device}"`);
    if (result.success) {
      return { status: 'booted', device: args.device };
    }
    // Check if already booted
    if (result.error?.includes('current state: Booted')) {
      return { status: 'already_booted', device: args.device };
    }
    return { error: result.error || result.stderr };
  },

  ios_shutdown_device: async (args) => {
    const result = simctl('shutdown', `"${args.device}"`);
    if (result.success) {
      return { status: 'shutdown', device: args.device };
    }
    if (result.error?.includes('current state: Shutdown')) {
      return { status: 'already_shutdown', device: args.device };
    }
    return { error: result.error || result.stderr };
  },

  ios_install_app: async (args) => {
    const result = simctl('install', `"${args.device}"`, `"${args.appPath}"`);
    if (result.success) {
      return { status: 'installed', device: args.device, app: args.appPath };
    }
    return { error: result.error || result.stderr };
  },

  ios_launch_app: async (args) => {
    const launchArgs = args.args ? args.args.join(' ') : '';
    const result = simctl('launch', `"${args.device}"`, args.bundleId, launchArgs);
    if (result.success) {
      // Output contains PID
      const pid = result.output.match(/(\d+)/)?.[1];
      return { status: 'launched', device: args.device, bundleId: args.bundleId, pid };
    }
    return { error: result.error || result.stderr };
  },

  ios_terminate_app: async (args) => {
    const result = simctl('terminate', `"${args.device}"`, args.bundleId);
    if (result.success) {
      return { status: 'terminated', device: args.device, bundleId: args.bundleId };
    }
    return { error: result.error || result.stderr };
  },

  ios_list_apps: async (args) => {
    const result = simctl('listapps', `"${args.device}"`);
    if (result.success) {
      // Parse plist output (simplified)
      try {
        const apps = [];
        const bundleIdMatches = result.output.matchAll(/CFBundleIdentifier\s*=\s*"([^"]+)"/g);
        const nameMatches = result.output.matchAll(/CFBundleDisplayName\s*=\s*"([^"]+)"/g);

        const bundleIds = [...bundleIdMatches].map(m => m[1]);
        const names = [...nameMatches].map(m => m[1]);

        for (let i = 0; i < bundleIds.length; i++) {
          apps.push({
            bundleId: bundleIds[i],
            name: names[i] || bundleIds[i]
          });
        }
        return { apps, count: apps.length };
      } catch (e) {
        return { rawOutput: result.output };
      }
    }
    return { error: result.error || result.stderr };
  },

  ios_screenshot: async (args) => {
    const outputPath = args.outputPath || path.join(os.tmpdir(), `ios-screenshot-${Date.now()}.png`);
    const result = simctl('io', `"${args.device}"`, 'screenshot', `"${outputPath}"`);
    if (result.success) {
      return { status: 'captured', path: outputPath };
    }
    return { error: result.error || result.stderr };
  },

  ios_open_url: async (args) => {
    const result = simctl('openurl', `"${args.device}"`, `"${args.url}"`);
    if (result.success) {
      return { status: 'opened', device: args.device, url: args.url };
    }
    return { error: result.error || result.stderr };
  },

  ios_set_location: async (args) => {
    const result = simctl('location', `"${args.device}"`, 'set', args.latitude.toString(), args.longitude.toString());
    if (result.success) {
      return { status: 'location_set', device: args.device, latitude: args.latitude, longitude: args.longitude };
    }
    return { error: result.error || result.stderr };
  },

  ios_push_notification: async (args) => {
    // Create temp file with payload
    const payloadPath = path.join(os.tmpdir(), `push-${Date.now()}.json`);
    fs.writeFileSync(payloadPath, JSON.stringify(args.payload));

    try {
      const result = simctl('push', `"${args.device}"`, args.bundleId, payloadPath);
      if (result.success) {
        return { status: 'sent', device: args.device, bundleId: args.bundleId };
      }
      return { error: result.error || result.stderr };
    } finally {
      fs.unlinkSync(payloadPath);
    }
  },

  ios_get_status: async () => {
    // Check Xcode
    let xcodeVersion = 'unknown';
    try {
      xcodeVersion = execSync('xcodebuild -version', { encoding: 'utf8' }).trim().split('\n')[0];
    } catch (e) {
      xcodeVersion = 'not installed or not configured';
    }

    // Get booted devices
    const devicesResult = simctlJson('list', 'devices');
    const bootedDevices = [];
    if (devicesResult.success) {
      for (const [runtime, runtimeDevices] of Object.entries(devicesResult.data.devices || {})) {
        for (const device of runtimeDevices) {
          if (device.state === 'Booted') {
            bootedDevices.push({
              name: device.name,
              udid: device.udid,
              runtime: runtime.replace('com.apple.CoreSimulator.SimRuntime.', '')
            });
          }
        }
      }
    }

    return {
      xcode: xcodeVersion,
      bootedDevices,
      bootedCount: bootedDevices.length
    };
  }
};

// SSE connection manager
const connections = new Map();
let connectionId = 0;

// Send SSE event to a connection
function sendEvent(res, event, data) {
  res.write(`event: ${event}\n`);
  res.write(`data: ${JSON.stringify(data)}\n\n`);
}

// Handle MCP requests
async function handleMcpRequest(req, res) {
  let body = '';
  for await (const chunk of req) {
    body += chunk;
  }

  try {
    const request = JSON.parse(body);
    const { id, method, params } = request;

    let result;
    switch (method) {
      case 'initialize':
        result = {
          protocolVersion: '2024-11-05',
          capabilities: { tools: {} },
          serverInfo: {
            name: 'hive-ios-mcp',
            version: '1.0.0',
            description: 'iOS Simulator automation via xcrun simctl'
          }
        };
        break;

      case 'tools/list':
        result = { tools: TOOLS };
        break;

      case 'tools/call':
        const { name, arguments: args } = params;
        const handler = toolHandlers[name];
        if (!handler) {
          res.statusCode = 404;
          res.end(JSON.stringify({ error: `Unknown tool: ${name}` }));
          return;
        }

        try {
          const toolResult = await handler(args || {});
          result = {
            content: [{ type: 'text', text: JSON.stringify(toolResult, null, 2) }]
          };
        } catch (error) {
          result = {
            content: [{ type: 'text', text: `Error: ${error.message}` }],
            isError: true
          };
        }
        break;

      default:
        res.statusCode = 404;
        res.end(JSON.stringify({ error: `Method not found: ${method}` }));
        return;
    }

    res.setHeader('Content-Type', 'application/json');
    res.end(JSON.stringify({ jsonrpc: '2.0', id, result }));
  } catch (error) {
    res.statusCode = 400;
    res.end(JSON.stringify({ error: `Parse error: ${error.message}` }));
  }
}

// Create HTTP server with SSE support
const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://localhost:${port}`);

  // CORS headers
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

  if (req.method === 'OPTIONS') {
    res.statusCode = 204;
    res.end();
    return;
  }

  // SSE endpoint
  if (url.pathname === '/sse' && req.method === 'GET') {
    res.setHeader('Content-Type', 'text/event-stream');
    res.setHeader('Cache-Control', 'no-cache');
    res.setHeader('Connection', 'keep-alive');

    const connId = ++connectionId;
    connections.set(connId, res);

    // Send initial connection event
    sendEvent(res, 'connected', { connectionId: connId });

    // Keep connection alive
    const keepAlive = setInterval(() => {
      res.write(':keepalive\n\n');
    }, 30000);

    req.on('close', () => {
      clearInterval(keepAlive);
      connections.delete(connId);
    });

    return;
  }

  // MCP request endpoint
  if (url.pathname === '/mcp' && req.method === 'POST') {
    await handleMcpRequest(req, res);
    return;
  }

  // Health check
  if (url.pathname === '/health') {
    res.setHeader('Content-Type', 'application/json');
    res.end(JSON.stringify({ status: 'ok', connections: connections.size }));
    return;
  }

  // 404 for other paths
  res.statusCode = 404;
  res.end('Not found');
});

// Start server
server.listen(port, () => {
  console.log(`HIVE iOS MCP Server running on http://localhost:${port}`);
  console.log(`SSE endpoint: http://localhost:${port}/sse`);
  console.log(`MCP endpoint: http://localhost:${port}/mcp`);
  console.log('');
  console.log('Available tools:');
  TOOLS.forEach(tool => {
    console.log(`  - ${tool.name}: ${tool.description}`);
  });
});

// Graceful shutdown
process.on('SIGINT', () => {
  console.log('\nShutting down...');
  server.close(() => {
    process.exit(0);
  });
});

process.on('SIGTERM', () => {
  server.close(() => {
    process.exit(0);
  });
});
