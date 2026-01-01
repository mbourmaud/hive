# Autonomous Testing

Drones can autonomously develop features AND test them using Playwright (browser) or iOS Simulator - no human intervention required.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                          macOS (Host)                                │
│                                                                      │
│  Playwright MCP (:8931)     iOS MCP (:8932)     iOS Simulator       │
│        │                         │                    │              │
│        │ browser_navigate        │ ios_open_url       │              │
│        ▼                         ▼                    ▼              │
│   localhost:13000 ◄─────────── exp://localhost:18081 ◄──────────────│
│        │                                                             │
└────────┼─────────────────────────────────────────────────────────────┘
         │ port mapping (3000→13000, 8081→18081)
┌────────┴─────────────────────────────────────────────────────────────┐
│                     Docker Container (drone-1)                        │
│                                                                       │
│  npm run dev → 0.0.0.0:3000    npx expo start → 0.0.0.0:8081        │
│                                                                       │
│  ENV: HIVE_EXPOSED_PORTS=3000:13000,8081:18081                       │
│                                                                       │
│  MCP Tool: hive_get_test_url(3000) → http://localhost:13000          │
│  MCP Tool: hive_get_test_url(8081, "exp") → exp://localhost:18081    │
└───────────────────────────────────────────────────────────────────────┘
```

> **Important**: Playwright and iOS Simulator run on the **host**, so they use `localhost:HOST_PORT`, not `host.docker.internal`. The drone just needs to know which host port its container port is mapped to.

## Configuration

### Step 1: Configure Port Mappings

Add port mappings to your `hive.yaml`:

```yaml
# hive.yaml
agents:
  workers:
    count: 2
    ports_per_drone:
      1: ["3000:13000", "8081:18081", "19000:19100"]
      2: ["3000:13001", "8081:18082", "19000:19200"]
```

This exposes:
- drone-1: container:3000 → host:13000
- drone-2: container:3000 → host:13001

### Step 2: Enable Host MCPs

```yaml
# hive.yaml
host_mcps:
  playwright:
    enabled: true
    port: 8931
    headless: false  # Watch tests run!
    browser: chromium
  ios:
    enabled: true
    port: 8932
```

### Step 3: Reinitialize Hive

```bash
hive clean
hive init
```

## Port Discovery

Drones discover their exposed ports using MCP tools:

### List All Ports

```
hive_list_exposed_ports()
```

Returns:
```json
{
  "success": true,
  "drone": "drone-1",
  "ports": [
    {
      "container": 3000,
      "host": 13000,
      "http_url": "http://localhost:13000",
      "expo_url": "exp://localhost:13000"
    }
  ]
}
```

### Get Specific URL

```
hive_get_test_url(port=3000)
```

Returns:
```json
{
  "success": true,
  "container_port": 3000,
  "host_port": 13000,
  "url": "http://localhost:13000"
}
```

For Expo apps:
```
hive_get_test_url(port=8081, protocol="exp")
```

Returns:
```json
{
  "url": "exp://localhost:18081"
}
```

## Autonomous Workflows

### Web App Testing

Complete autonomous workflow:

```
1. Drone implements feature
2. Drone: npm run dev (port 3000)
3. Drone: hive_get_test_url(3000) → http://localhost:13000
4. Drone uses Playwright MCP:
   - browser_navigate("http://localhost:13000")
   - browser_snapshot()
   - browser_click/type to test
   - browser_screenshot() for evidence
5. Drone: hive_complete_task("Feature tested, see screenshots")
```

### Expo/React Native Testing

```
1. Drone implements feature
2. Drone: npx expo start --port 8081
3. Drone: hive_get_test_url(8081, protocol="exp") → exp://localhost:18081
4. Drone uses iOS MCP:
   - ios_list_devices() → find iPhone 15
   - ios_boot_device("iPhone 15")
   - ios_open_url("booted", "exp://localhost:18081")
   - ios_screenshot("booted")
5. Drone: hive_complete_task("Feature tested on iOS Simulator")
```

### Container Headless Playwright

For fast CI-style testing without host MCP:

```javascript
// Playwright is installed at /opt/playwright
const { chromium } = require('playwright');
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
await page.goto('http://localhost:3000'); // Container localhost
await page.screenshot({ path: 'test.png' });
await browser.close();
```

## Environment Variables

These environment variables are automatically set for drones with port mappings:

| Variable | Description | Example |
|----------|-------------|---------|
| `HIVE_EXPOSED_PORTS` | Comma-separated port mappings | `3000:13000,8081:18081` |

> **Note**: URLs use `localhost:HOST_PORT` because Playwright and iOS Simulator run on the host machine, not inside Docker.

## MCP Tools Reference

### Drone Testing Tools

| Tool | Description |
|------|-------------|
| `hive_get_test_url` | Get accessible URL for a container port |
| `hive_list_exposed_ports` | List all exposed port mappings |

### Playwright MCP Tools (Host)

| Tool | Description |
|------|-------------|
| `browser_navigate` | Navigate to URL |
| `browser_snapshot` | Get accessibility tree |
| `browser_click` | Click on element |
| `browser_type` | Type into input |
| `browser_screenshot` | Take screenshot |

### iOS MCP Tools (Host)

| Tool | Description |
|------|-------------|
| `ios_list_devices` | List available simulators |
| `ios_boot_device` | Start a simulator |
| `ios_open_url` | Open URL in simulator |
| `ios_screenshot` | Take simulator screenshot |

## Troubleshooting

### "No exposed ports configured"

Add `ports_per_drone` to your `hive.yaml` and reinitialize:

```yaml
agents:
  workers:
    ports_per_drone:
      1: ["3000:13000"]
```

```bash
hive clean && hive init
```

### Container app not accessible from host

1. Ensure app binds to `0.0.0.0`, not `127.0.0.1`:
   ```bash
   npm run dev -- --host 0.0.0.0
   # or
   npx expo start --host 0.0.0.0
   ```

2. Check port mapping in `hive.yaml`

3. Verify container is running:
   ```bash
   docker ps
   ```

### Playwright MCP can't reach app

1. Check MCP is running: `hive mcp status`
2. Verify URL uses `localhost:HOST_PORT` (e.g., `localhost:13000`, not `localhost:3000`)
3. Check host port is correct (the second number in the mapping)

### iOS Simulator not opening Expo

1. Ensure Expo Go is installed in simulator: `ios_install_expo_go(device="iPhone 15")`
2. Check URL format: `exp://localhost:HOST_PORT` (e.g., `exp://localhost:18081`)
3. Verify Metro bundler is running on the container port
4. Make sure `EXPO_PACKAGER_PROXY_URL` is set in `.env`
