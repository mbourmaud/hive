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

// Tool: ios_install_expo_go
server.tool(
  'ios_install_expo_go',
  { device: z.string().describe('Device UDID or name to install Expo Go on') },
  async ({ device }) => {
    const tempDir = os.tmpdir();
    const appPath = path.join(tempDir, 'Exponent.app');
    const tarPath = path.join(tempDir, 'expo-go.tar.gz');

    try {
      // Get latest Expo Go version from Expo's versions endpoint
      console.log('[ios_install_expo_go] Fetching latest Expo Go version...');
      let version = '2.32.18'; // Default fallback version

      try {
        const versionsResponse = execSync('curl -s "https://expo.dev/--/api/v2/versions"', {
          encoding: 'utf8',
          timeout: 10000
        });
        const versionsData = JSON.parse(versionsResponse);
        if (versionsData.data?.sdkVersions) {
          // Get the latest SDK version's iosClientVersion
          const sdkVersions = Object.keys(versionsData.data.sdkVersions).sort().reverse();
          for (const sdk of sdkVersions) {
            const clientVersion = versionsData.data.sdkVersions[sdk]?.iosClientVersion;
            if (clientVersion) {
              version = clientVersion;
              console.log(`[ios_install_expo_go] Found Expo Go version ${version} for SDK ${sdk}`);
              break;
            }
          }
        }
      } catch (e) {
        console.log('[ios_install_expo_go] Could not fetch version, using fallback');
      }

      // Use Expo's official CDN for iOS simulator builds
      const downloadUrl = `https://dpq5q02fu5f55.cloudfront.net/Exponent-${version}.tar.gz`;

      // Download Expo Go
      console.log(`[ios_install_expo_go] Downloading Expo Go ${version}...`);
      execSync(`curl -L -f -o "${tarPath}" "${downloadUrl}"`, {
        encoding: 'utf8',
        maxBuffer: 100 * 1024 * 1024,
        timeout: 120000
      });

      // Extract to a temp subdirectory (ignore timestamp errors on macOS)
      console.log('[ios_install_expo_go] Extracting...');
      const extractDir = path.join(tempDir, `expo-extract-${Date.now()}`);
      const finalAppPath = path.join(tempDir, 'Exponent.app');

      // Clean up any existing directories
      if (fs.existsSync(extractDir)) execSync(`rm -rf "${extractDir}"`);
      if (fs.existsSync(finalAppPath)) execSync(`rm -rf "${finalAppPath}"`);

      // Create extraction directory
      fs.mkdirSync(extractDir, { recursive: true });

      try {
        execSync(`tar -xzf "${tarPath}" -C "${extractDir}" --no-same-permissions 2>/dev/null || tar -xzf "${tarPath}" -C "${extractDir}" 2>/dev/null || true`, { encoding: 'utf8' });
      } catch (e) {
        // Ignore tar timestamp errors - files are still extracted
      }

      // Check if tar extracted a .app directory or just the contents
      const extractedFiles = fs.readdirSync(extractDir);
      const existingApp = extractedFiles.find(f => f.endsWith('.app'));

      if (existingApp) {
        // Tar contained a .app folder - just rename it
        fs.renameSync(path.join(extractDir, existingApp), finalAppPath);
      } else {
        // Tar contained the bundle contents directly - wrap in .app folder
        console.log('[ios_install_expo_go] Wrapping extracted files in Exponent.app bundle...');
        fs.renameSync(extractDir, finalAppPath);
      }

      // Verify we have the app bundle
      if (!fs.existsSync(finalAppPath) || !fs.existsSync(path.join(finalAppPath, 'Info.plist'))) {
        throw new Error('Could not create valid Expo Go app bundle');
      }

      // Install on simulator
      console.log(`[ios_install_expo_go] Installing on device ${device}...`);
      const installResult = simctl('install', `"${device}"`, `"${finalAppPath}"`);

      // Cleanup
      fs.unlinkSync(tarPath);
      execSync(`rm -rf "${finalAppPath}"`);

      if (installResult.success) {
        return {
          content: [{
            type: 'text',
            text: JSON.stringify({
              status: 'installed',
              device,
              app: 'Expo Go',
              bundleId: 'host.exp.Exponent',
              note: 'Use ios_launch_app with bundleId "host.exp.Exponent" to launch Expo Go'
            }, null, 2)
          }]
        };
      }
      return { content: [{ type: 'text', text: JSON.stringify({ error: installResult.error || installResult.stderr }) }], isError: true };
    } catch (error) {
      // Cleanup on error
      if (fs.existsSync(tarPath)) fs.unlinkSync(tarPath);
      if (fs.existsSync(appPath)) execSync(`rm -rf "${appPath}"`);

      return {
        content: [{ type: 'text', text: JSON.stringify({ error: `Failed to install Expo Go: ${error.message}` }) }],
        isError: true
      };
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
  console.log('  - ios_install_expo_go: Download and install Expo Go');
  console.log('  - ios_get_status: Get Xcode and simulator status');
});
