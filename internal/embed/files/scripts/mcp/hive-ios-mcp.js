#!/usr/bin/env node
/**
 * HIVE iOS MCP Server
 *
 * Provides iOS Simulator automation via xcrun simctl
 * Uses @modelcontextprotocol/sdk with SSE transport
 */

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { SSEServerTransport } from '@modelcontextprotocol/sdk/server/sse.js';
import express from 'express';
import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import os from 'os';
import { z } from 'zod';

// Parse command line arguments
const args = process.argv.slice(2);
let port = 8932;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--port' && args[i + 1]) {
    port = parseInt(args[i + 1], 10);
  }
}

// Execute xcrun simctl command
function simctl(...cmdArgs) {
  try {
    const result = execSync(`xcrun simctl ${cmdArgs.join(' ')}`, {
      encoding: 'utf8',
      maxBuffer: 10 * 1024 * 1024
    });
    return { success: true, output: result.trim() };
  } catch (error) {
    return { success: false, error: error.message, stderr: error.stderr?.toString() };
  }
}

// Execute simctl with JSON output
function simctlJson(...cmdArgs) {
  const result = simctl(...cmdArgs, '-j');
  if (result.success) {
    try {
      return { success: true, data: JSON.parse(result.output) };
    } catch (e) {
      return { success: false, error: 'Failed to parse JSON output' };
    }
  }
  return result;
}

// Create MCP Server
const server = new McpServer({
  name: 'hive-ios-mcp',
  version: '1.0.0'
});

// Tool: ios_list_devices
server.tool(
  'ios_list_devices',
  {
    filter: z.enum(['all', 'booted', 'available']).optional().default('all').describe('Filter devices: all (default), booted (running only), available (installed only)')
  },
  async ({ filter }) => {
    const result = simctlJson('list', 'devices');
    if (!result.success) {
      return { content: [{ type: 'text', text: JSON.stringify({ error: result.error }) }], isError: true };
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

    return { content: [{ type: 'text', text: JSON.stringify({ devices, count: devices.length }, null, 2) }] };
  }
);

// Tool: ios_boot_device
server.tool(
  'ios_boot_device',
  { device: z.string().describe('Device UDID or name') },
  async ({ device }) => {
    const result = simctl('boot', `"${device}"`);
    if (result.success) {
      return { content: [{ type: 'text', text: JSON.stringify({ status: 'booted', device }) }] };
    }
    if (result.error?.includes('current state: Booted')) {
      return { content: [{ type: 'text', text: JSON.stringify({ status: 'already_booted', device }) }] };
    }
    return { content: [{ type: 'text', text: JSON.stringify({ error: result.error || result.stderr }) }], isError: true };
  }
);

// Tool: ios_shutdown_device
server.tool(
  'ios_shutdown_device',
  { device: z.string().describe('Device UDID or name') },
  async ({ device }) => {
    const result = simctl('shutdown', `"${device}"`);
    if (result.success) {
      return { content: [{ type: 'text', text: JSON.stringify({ status: 'shutdown', device }) }] };
    }
    if (result.error?.includes('current state: Shutdown')) {
      return { content: [{ type: 'text', text: JSON.stringify({ status: 'already_shutdown', device }) }] };
    }
    return { content: [{ type: 'text', text: JSON.stringify({ error: result.error || result.stderr }) }], isError: true };
  }
);

// Tool: ios_install_app
server.tool(
  'ios_install_app',
  {
    device: z.string().describe('Device UDID or name'),
    appPath: z.string().describe('Path to .app bundle')
  },
  async ({ device, appPath }) => {
    const result = simctl('install', `"${device}"`, `"${appPath}"`);
    if (result.success) {
      return { content: [{ type: 'text', text: JSON.stringify({ status: 'installed', device, app: appPath }) }] };
    }
    return { content: [{ type: 'text', text: JSON.stringify({ error: result.error || result.stderr }) }], isError: true };
  }
);

// Tool: ios_launch_app
server.tool(
  'ios_launch_app',
  {
    device: z.string().describe('Device UDID or name'),
    bundleId: z.string().describe('App bundle identifier (e.g., com.example.app)'),
    launchArgs: z.array(z.string()).optional().describe('Optional launch arguments')
  },
  async ({ device, bundleId, launchArgs }) => {
    const extraArgs = launchArgs ? launchArgs.join(' ') : '';
    const result = simctl('launch', `"${device}"`, bundleId, extraArgs);
    if (result.success) {
      const pid = result.output.match(/(\d+)/)?.[1];
      return { content: [{ type: 'text', text: JSON.stringify({ status: 'launched', device, bundleId, pid }) }] };
    }
    return { content: [{ type: 'text', text: JSON.stringify({ error: result.error || result.stderr }) }], isError: true };
  }
);

// Tool: ios_terminate_app
server.tool(
  'ios_terminate_app',
  {
    device: z.string().describe('Device UDID or name'),
    bundleId: z.string().describe('App bundle identifier')
  },
  async ({ device, bundleId }) => {
    const result = simctl('terminate', `"${device}"`, bundleId);
    if (result.success) {
      return { content: [{ type: 'text', text: JSON.stringify({ status: 'terminated', device, bundleId }) }] };
    }
    return { content: [{ type: 'text', text: JSON.stringify({ error: result.error || result.stderr }) }], isError: true };
  }
);

// Tool: ios_list_apps
server.tool(
  'ios_list_apps',
  { device: z.string().describe('Device UDID or name') },
  async ({ device }) => {
    const result = simctl('listapps', `"${device}"`);
    if (result.success) {
      try {
        const apps = [];
        const bundleIdMatches = result.output.matchAll(/CFBundleIdentifier\s*=\s*"([^"]+)"/g);
        const nameMatches = result.output.matchAll(/CFBundleDisplayName\s*=\s*"([^"]+)"/g);
        const bundleIds = [...bundleIdMatches].map(m => m[1]);
        const names = [...nameMatches].map(m => m[1]);
        for (let i = 0; i < bundleIds.length; i++) {
          apps.push({ bundleId: bundleIds[i], name: names[i] || bundleIds[i] });
        }
        return { content: [{ type: 'text', text: JSON.stringify({ apps, count: apps.length }, null, 2) }] };
      } catch (e) {
        return { content: [{ type: 'text', text: result.output }] };
      }
    }
    return { content: [{ type: 'text', text: JSON.stringify({ error: result.error || result.stderr }) }], isError: true };
  }
);

// Tool: ios_screenshot
server.tool(
  'ios_screenshot',
  {
    device: z.string().describe('Device UDID or name'),
    outputPath: z.string().optional().describe('Output file path (optional, defaults to temp file)')
  },
  async ({ device, outputPath }) => {
    const finalPath = outputPath || path.join(os.tmpdir(), `ios-screenshot-${Date.now()}.png`);
    const result = simctl('io', `"${device}"`, 'screenshot', `"${finalPath}"`);
    if (result.success) {
      return { content: [{ type: 'text', text: JSON.stringify({ status: 'captured', path: finalPath }) }] };
    }
    return { content: [{ type: 'text', text: JSON.stringify({ error: result.error || result.stderr }) }], isError: true };
  }
);

// Tool: ios_open_url
server.tool(
  'ios_open_url',
  {
    device: z.string().describe('Device UDID or name'),
    url: z.string().describe('URL to open')
  },
  async ({ device, url }) => {
    const result = simctl('openurl', `"${device}"`, `"${url}"`);
    if (result.success) {
      return { content: [{ type: 'text', text: JSON.stringify({ status: 'opened', device, url }) }] };
    }
    return { content: [{ type: 'text', text: JSON.stringify({ error: result.error || result.stderr }) }], isError: true };
  }
);

// Tool: ios_set_location
server.tool(
  'ios_set_location',
  {
    device: z.string().describe('Device UDID or name'),
    latitude: z.number().describe('Latitude coordinate'),
    longitude: z.number().describe('Longitude coordinate')
  },
  async ({ device, latitude, longitude }) => {
    const result = simctl('location', `"${device}"`, 'set', latitude.toString(), longitude.toString());
    if (result.success) {
      return { content: [{ type: 'text', text: JSON.stringify({ status: 'location_set', device, latitude, longitude }) }] };
    }
    return { content: [{ type: 'text', text: JSON.stringify({ error: result.error || result.stderr }) }], isError: true };
  }
);

// Tool: ios_push_notification
server.tool(
  'ios_push_notification',
  {
    device: z.string().describe('Device UDID or name'),
    bundleId: z.string().describe('App bundle identifier'),
    payload: z.record(z.unknown()).describe('APNS payload object (e.g., { aps: { alert: "Hello" } })')
  },
  async ({ device, bundleId, payload }) => {
    const payloadPath = path.join(os.tmpdir(), `push-${Date.now()}.json`);
    fs.writeFileSync(payloadPath, JSON.stringify(payload));
    try {
      const result = simctl('push', `"${device}"`, bundleId, payloadPath);
      if (result.success) {
        return { content: [{ type: 'text', text: JSON.stringify({ status: 'sent', device, bundleId }) }] };
      }
      return { content: [{ type: 'text', text: JSON.stringify({ error: result.error || result.stderr }) }], isError: true };
    } finally {
      fs.unlinkSync(payloadPath);
    }
  }
);

// Tool: ios_get_status (no parameters)
server.tool(
  'ios_get_status',
  {},
  async () => {
    let xcodeVersion = 'unknown';
    try {
      xcodeVersion = execSync('xcodebuild -version', { encoding: 'utf8' }).trim().split('\n')[0];
    } catch (e) {
      xcodeVersion = 'not installed or not configured';
    }

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
      content: [{
        type: 'text',
        text: JSON.stringify({ xcode: xcodeVersion, bootedDevices, bootedCount: bootedDevices.length }, null, 2)
      }]
    };
  }
);

// Express server with SSE transport
const app = express();
app.use(express.json());

// Track active transports
const transports = new Map();

app.get('/sse', async (req, res) => {
  const transport = new SSEServerTransport('/messages', res);
  // Use the sessionId generated by SSEServerTransport (UUID sent to client)
  const sessionId = transport.sessionId;
  transports.set(sessionId, transport);

  res.on('close', () => {
    transports.delete(sessionId);
  });

  await server.connect(transport);
});

app.post('/messages', async (req, res) => {
  const sessionId = req.query.sessionId;
  const transport = sessionId ? transports.get(sessionId) : [...transports.values()].pop();

  if (transport) {
    await transport.handlePostMessage(req, res, req.body);
  } else {
    res.status(400).json({ error: 'No active SSE connection' });
  }
});

app.get('/health', (req, res) => {
  res.json({ status: 'ok', connections: transports.size });
});

app.listen(port, () => {
  console.log(`HIVE iOS MCP Server running on http://localhost:${port}`);
  console.log(`SSE endpoint: http://localhost:${port}/sse`);
  console.log(`Messages endpoint: http://localhost:${port}/messages`);
  console.log('');
  console.log('Available tools:');
  console.log('  - ios_list_devices: List available simulators');
  console.log('  - ios_boot_device: Boot a simulator');
  console.log('  - ios_shutdown_device: Shutdown a simulator');
  console.log('  - ios_install_app: Install .app bundle');
  console.log('  - ios_launch_app: Launch app by bundle ID');
  console.log('  - ios_terminate_app: Stop running app');
  console.log('  - ios_list_apps: List installed apps');
  console.log('  - ios_screenshot: Take screenshot');
  console.log('  - ios_open_url: Open URL in Safari');
  console.log('  - ios_set_location: Set GPS location');
  console.log('  - ios_push_notification: Send push notification');
  console.log('  - ios_get_status: Get Xcode and simulator status');
});
