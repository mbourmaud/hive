# Host MCPs - Browser, iOS & Clipboard

Host MCPs are MCP servers that run on your macOS host machine (not inside Docker containers). They provide browser automation, iOS Simulator control, and clipboard access to Claude Code agents running in containers.

## Architecture

```
┌───────────────────────────────────────────────────────────────────────────┐
│                              HOST (macOS)                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐           │
│  │ Playwright MCP  │  │  iOS MCP        │  │ Clipboard MCP   │           │
│  │ :8931/sse       │  │  :8932/sse      │  │ :8933/sse       │           │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘           │
│           │                    │                     │                    │
│           ▼                    ▼                     ▼                    │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐           │
│  │ Chrome/Safari   │  │ iOS Simulator   │  │ macOS Clipboard │           │
│  │ Browser         │  │ (Xcode)         │  │ (pbcopy/paste)  │           │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘           │
│                                                                           │
└─────────────────────────────┬─────────────────────────────────────────────┘
                              │ host.docker.internal
┌─────────────────────────────┴─────────────────────────────────────────────┐
│                          Docker Network                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                    │
│  │    Queen     │  │   Drone-1    │  │   Drone-2    │                    │
│  │ Claude Code  │  │ Claude Code  │  │ Claude Code  │                    │
│  │              │  │              │  │              │                    │
│  │ MCP Client → │  │ MCP Client → │  │ MCP Client → │                    │
│  │ host:8931+   │  │ host:8931+   │  │ host:8931+   │                    │
│  └──────────────┘  └──────────────┘  └──────────────┘                    │
└───────────────────────────────────────────────────────────────────────────┘
```

## Configuration

Add the `host_mcps` section to your `hive.yaml`:

```yaml
# hive.yaml
workspace:
  name: my-app

host_mcps:
  # Browser automation via Playwright
  playwright:
    enabled: true
    port: 8931              # SSE port (default: 8931)
    headless: false         # false = visible browser you can watch
    browser: chromium       # chromium, firefox, or webkit

  # iOS Simulator automation
  ios:
    enabled: true
    port: 8932              # SSE port (default: 8932)

  # Clipboard access (text and images)
  clipboard:
    enabled: true
    port: 8933              # SSE port (default: 8933)
```

## Usage

### Automatic Lifecycle

When you run `hive start`, host MCPs are automatically started before Docker containers:

```bash
hive start 2
# 🌐 Starting Playwright MCP on port 8931...
# 📱 Starting iOS MCP on port 8932...
# 📋 Starting Clipboard MCP on port 8933...
# ✓ Host MCPs running: [playwright (port 8931), ios (port 8932), clipboard (port 8933)]
# 🚀 Starting Hive
# ...
```

When you run `hive stop`, host MCPs are stopped after Docker containers:

```bash
hive stop
# 🛑 Stopping Hive
# ...
# 🔌 Stopping host MCPs...
# ✓ Host MCPs stopped
```

### Manual Control

You can manually manage host MCPs:

```bash
# Check status
hive mcp status

# List all configured MCPs
hive mcp list

# Start/stop individually
hive mcp start playwright
hive mcp stop ios

# View logs
hive mcp logs playwright -n 100
```

## Playwright MCP

### Prerequisites

- Node.js 18+ installed on host
- `npx` available in PATH

### Available Tools

When Playwright MCP is running, Claude Code agents get access to these browser automation tools:

- `browser_navigate` - Navigate to a URL
- `browser_click` - Click on elements
- `browser_type` - Type text into inputs
- `browser_snapshot` - Capture page accessibility tree
- `browser_screenshot` - Take screenshots
- `browser_console_messages` - Get console logs
- `browser_evaluate` - Execute JavaScript
- And more...

### Example Use Case

```
You: Open my app at localhost:3000 and test the login flow

Claude: I'll use the browser automation tools to test your login flow.
[Uses browser_navigate to open localhost:3000]
[Uses browser_snapshot to understand the page]
[Uses browser_type to enter credentials]
[Uses browser_click to submit]
[Uses browser_screenshot to capture result]
```

### Headless vs Visible Mode

- **Headless mode** (`headless: true`): Browser runs invisibly, faster execution
- **Visible mode** (`headless: false`): You can watch Claude interact with the browser

## iOS MCP

### Prerequisites

- macOS only
- Xcode installed (includes Simulator)
- Xcode Command Line Tools (`xcode-select --install`)

### Available Tools

- `ios_list_devices` - List available simulators
- `ios_boot_device` - Start a simulator
- `ios_shutdown_device` - Stop a simulator
- `ios_install_app` - Install .app bundle
- `ios_launch_app` - Launch app by bundle ID
- `ios_terminate_app` - Stop running app
- `ios_screenshot` - Take simulator screenshot
- `ios_open_url` - Open URL in Safari
- `ios_set_location` - Set GPS coordinates
- `ios_push_notification` - Send test push notifications
- `ios_get_status` - Get Xcode/simulator status

### Example Use Case

```
You: Test my iOS app on iPhone 15 simulator

Claude: I'll set up the simulator and test your app.
[Uses ios_list_devices to find iPhone 15]
[Uses ios_boot_device to start it]
[Uses ios_install_app to install your .app]
[Uses ios_launch_app to start the app]
[Uses ios_screenshot to capture the UI]
```

## Clipboard MCP

### Prerequisites

- macOS only
- Node.js 18+ installed on host
- For image support: `pngpaste` (`brew install pngpaste`)

### Available Tools

- `clipboard_read_text` - Read text from the macOS clipboard
- `clipboard_write_text` - Write text to the clipboard
- `clipboard_read_image` - Read image from clipboard (returns base64 PNG)
- `clipboard_write_image` - Write image to clipboard (from base64 or file path)
- `clipboard_get_formats` - List available data formats in clipboard
- `clipboard_clear` - Clear the clipboard

### Example Use Case

```
You: I copied some code, please review it

Claude: I'll read the code from your clipboard.
[Uses clipboard_read_text to get the code]
Here's my review of the code you copied...

You: I took a screenshot of a bug, can you analyze it?

Claude: I'll look at the image from your clipboard.
[Uses clipboard_read_image to get the screenshot]
I can see the bug in your screenshot. The issue is...
```

### Image Support

For full image clipboard support, install `pngpaste`:

```bash
brew install pngpaste
```

Without `pngpaste`, only text clipboard operations will work.

## Troubleshooting

### Playwright MCP won't start

1. Check Node.js is installed: `node --version`
2. Check npx is available: `npx --version`
3. View logs: `hive mcp logs playwright`

### iOS MCP won't start

1. Check Xcode is installed: `xcodebuild -version`
2. Check simctl works: `xcrun simctl list devices`
3. View logs: `hive mcp logs ios`
4. Note: Only works on macOS

### Clipboard MCP won't start

1. Check Node.js is installed: `node --version`
2. View logs: `hive mcp logs clipboard`
3. Note: Only works on macOS

### Clipboard image support not working

1. Install pngpaste: `brew install pngpaste`
2. Verify installation: `which pngpaste`
3. Test manually: `pngpaste -` (should output image data if clipboard has an image)

### Port already in use

If you get "port XXXX is already in use":

1. Check what's using it: `lsof -i :8931`
2. Kill the process or change the port in `hive.yaml`

### Container can't connect to host MCP

1. Ensure `host.docker.internal` resolves inside container
2. Check MCP is running: `hive mcp status`
3. Test connectivity: `curl http://host.docker.internal:8931/health`

## How It Works

1. **On `hive start`**: Hive starts MCP servers as background processes on the host
2. **MCP servers**: Listen on specified ports using SSE (Server-Sent Events) transport
3. **Docker compose**: Passes `HOST_MCP_PLAYWRIGHT_PORT`, `HOST_MCP_IOS_PORT`, and `HOST_MCP_CLIPBOARD_PORT` environment variables
4. **Container entrypoint**: Registers SSE MCPs in Claude Code's config
5. **Claude Code**: Connects to `http://host.docker.internal:PORT/sse`
6. **On `hive stop`**: Host MCP processes are terminated

## Files

| File | Description |
|------|-------------|
| `.hive/pids/playwright.pid` | Playwright MCP process ID |
| `.hive/pids/ios.pid` | iOS MCP process ID |
| `.hive/pids/clipboard.pid` | Clipboard MCP process ID |
| `.hive/logs/playwright.log` | Playwright MCP logs |
| `.hive/logs/ios.log` | iOS MCP logs |
| `.hive/logs/clipboard.log` | Clipboard MCP logs |
