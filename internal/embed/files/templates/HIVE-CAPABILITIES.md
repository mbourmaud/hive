# HIVE Capabilities Reference

This document describes ALL capabilities available to you as a Hive agent. Read this carefully to understand what tools you have access to.

---

## Host MCPs (Running on macOS Host)

You have access to three powerful MCPs running on the host machine. These give you control over browser automation, iOS Simulator, and the system clipboard.

### Playwright MCP (Browser Automation)

Control a real browser on the host machine. You can navigate, click, type, take screenshots, and automate web testing.

**Available Tools:**

| Tool | Description |
|------|-------------|
| `browser_navigate` | Navigate to a URL |
| `browser_click` | Click on an element |
| `browser_type` | Type text into an input |
| `browser_snapshot` | Get accessibility tree (understand the page) |
| `browser_screenshot` | Take a screenshot |
| `browser_evaluate` | Execute JavaScript |
| `browser_console_messages` | Get console logs |
| `browser_network_requests` | See network activity |
| `browser_tabs` | Manage browser tabs |
| `browser_wait_for` | Wait for text/element |
| `browser_hover` | Hover over element |
| `browser_drag` | Drag and drop |
| `browser_select_option` | Select from dropdown |
| `browser_fill_form` | Fill multiple form fields |
| `browser_file_upload` | Upload files |
| `browser_press_key` | Press keyboard key |

**Example - Testing a Login Flow:**
```
browser_navigate(url="http://host.docker.internal:13000")
browser_snapshot()  # Understand the page structure
browser_type(element="email input", ref="input[name=email]", text="user@example.com")
browser_type(element="password input", ref="input[name=password]", text="secret123")
browser_click(element="login button", ref="button[type=submit]")
browser_wait_for(text="Welcome")
browser_screenshot()  # Capture evidence
```

### iOS MCP (iOS Simulator Control)

Control iOS Simulators on the host. Perfect for testing React Native, Expo, or native iOS apps.

**Available Tools:**

| Tool | Description |
|------|-------------|
| `ios_list_devices` | List available simulators |
| `ios_boot_device` | Start a simulator |
| `ios_shutdown_device` | Stop a simulator |
| `ios_install_app` | Install .app bundle |
| `ios_launch_app` | Launch app by bundle ID |
| `ios_terminate_app` | Stop running app |
| `ios_screenshot` | Take simulator screenshot |
| `ios_open_url` | Open URL in Safari/Expo Go |
| `ios_set_location` | Set GPS coordinates |
| `ios_push_notification` | Send test notifications |
| `ios_get_status` | Get Xcode/simulator status |

**Example - Testing an Expo App:**
```
ios_list_devices()  # Find available simulators
ios_boot_device(deviceId="iPhone 15")
ios_open_url(deviceId="booted", url="exp://host.docker.internal:18081")
ios_screenshot(deviceId="booted")
```

### Clipboard MCP (System Clipboard)

Read and write to the macOS system clipboard. Perfect for sharing data between you and the user.

**Available Tools:**

| Tool | Description |
|------|-------------|
| `clipboard_read_text` | Read text from clipboard |
| `clipboard_write_text` | Write text to clipboard |
| `clipboard_read_image` | Read image from clipboard (base64 PNG) |
| `clipboard_write_image` | Write image to clipboard |
| `clipboard_get_formats` | List available clipboard formats |
| `clipboard_clear` | Clear the clipboard |

**Example - Reading User's Clipboard:**
```
clipboard_read_text()  # Get what the user copied
# Now you can analyze or use the clipboard content
```

---

## Hive MCP Tools

You have access to Hive-specific tools for task management and coordination.

### Task Management (Workers)

| Tool | Description |
|------|-------------|
| `hive_my_tasks` | Get your current and queued tasks |
| `hive_take_task` | Start working on the next queued task |
| `hive_complete_task` | Mark current task as done |
| `hive_fail_task` | Mark current task as failed |
| `hive_log_activity` | Log progress for Queen visibility |

### Task Assignment (Queen)

| Tool | Description |
|------|-------------|
| `hive_assign_task` | Assign a task to a drone |
| `hive_list_drones` | List all drones and their status |
| `hive_drone_status` | Get detailed status of a drone |
| `hive_get_drone_logs` | Read a drone's activity logs |

### Configuration & Monitoring

| Tool | Description |
|------|-------------|
| `hive_get_config` | Read hive.yaml configuration |
| `hive_start_monitoring` | Start background task monitoring |
| `hive_get_monitoring_events` | Get pending monitoring events |

### Autonomous Testing Tools

| Tool | Description |
|------|-------------|
| `hive_get_test_url` | Get host-accessible URL for a container port |
| `hive_list_exposed_ports` | List all exposed port mappings |

---

## Autonomous Testing

You can develop features AND test them autonomously using the Host MCPs.

### Port Discovery

Your container ports are mapped to host ports. Use these tools to discover them:

```
hive_list_exposed_ports()
# Returns: { ports: [{ container: 3000, host: 13000, http_url: "http://..." }] }

hive_get_test_url(port=3000)
# Returns: { url: "http://host.docker.internal:13000" }

hive_get_test_url(port=8081, protocol="exp")
# Returns: { url: "exp://host.docker.internal:18081" }
```

### Web App Testing Workflow

1. **Implement the feature**
2. **Start your dev server:**
   ```bash
   npm run dev &
   hive-log "🚀 SERVER RUNNING: http://localhost:3000 (frontend)"
   ```

3. **Get the test URL:**
   ```
   hive_get_test_url(port=3000) → { url: "http://host.docker.internal:13000" }
   ```

4. **Test with Playwright:**
   ```
   browser_navigate(url="http://host.docker.internal:13000")
   browser_snapshot()
   browser_type(element="...", text="...")
   browser_click(element="...")
   browser_screenshot()
   ```

5. **Complete the task:**
   ```
   hive_complete_task(result="Feature implemented and tested. Screenshots attached.")
   ```

### Expo/React Native Testing Workflow

1. **Implement the feature**
2. **Start Metro bundler:**
   ```bash
   npx expo start --port 8081 &
   hive-log "🚀 SERVER RUNNING: http://localhost:8081 (Metro bundler)"
   ```

3. **Get the Expo URL:**
   ```
   hive_get_test_url(port=8081, protocol="exp") → { url: "exp://host.docker.internal:18081" }
   ```

4. **Test on iOS Simulator:**
   ```
   ios_list_devices()
   ios_boot_device(deviceId="iPhone 15")
   ios_open_url(deviceId="booted", url="exp://host.docker.internal:18081")
   ios_screenshot(deviceId="booted")
   ```

5. **Complete the task:**
   ```
   hive_complete_task(result="Feature implemented and tested on iOS Simulator.")
   ```

### Headless Testing in Container

For fast CI-style testing without host MCP, use Playwright directly:

```javascript
const { chromium } = require('playwright');

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
await page.goto('http://localhost:3000');  // Container localhost
await page.screenshot({ path: 'test.png' });
await browser.close();
```

Playwright is pre-installed at `/opt/playwright` in the container.

---

## Important URLs

When running servers in your container, you need to use the correct URL format:

| Context | URL Format |
|---------|------------|
| Inside container (localhost) | `http://localhost:3000` |
| From host to container | `http://host.docker.internal:HOST_PORT` |
| Expo deep link | `exp://host.docker.internal:HOST_PORT` |

**Use `hive_get_test_url()` to get the correct host-accessible URL!**

---

## Best Practices

### Always Log Your Activity

Use `hive_log_activity()` or `hive-log` to keep the Queen informed:

```bash
hive-log "📋 Starting: Add login form"
hive-log "📖 Reading: src/api/auth.ts"
hive-log "✏️ Editing: src/components/Login.tsx"
hive-log "🚀 SERVER RUNNING: http://localhost:3000 (frontend)"
hive-log "🧪 Testing login flow with Playwright"
hive-log "✅ Task completed, PR created"
```

### Log Emojis

| Emoji | Meaning |
|-------|---------|
| 📋 | Starting task |
| 📖 | Reading files |
| ✏️ | Editing files |
| 🔨 | Running commands |
| 🚀 | Starting server |
| 🧪 | Running tests |
| ⏳ | Waiting (CI, build) |
| ✅ | Success |
| 🚫 | Blocked |
| ❌ | Error |

### Server Port Logging

When you start a dev server, ALWAYS log it with this format:
```
🚀 SERVER RUNNING: http://localhost:PORT (description)
```

This allows the Queen to know about running servers and helps with `hive expose`.

### Autonomous Testing Checklist

1. Start the app/server
2. Log the running server
3. Get the test URL with `hive_get_test_url()`
4. Use Playwright or iOS MCP to test
5. Take screenshots as evidence
6. Include results in `hive_complete_task()`

---

## Environment Variables

These environment variables are available in your container:

| Variable | Description |
|----------|-------------|
| `AGENT_ROLE` | Your role: `queen` or `worker` |
| `AGENT_NAME` | Your name: `queen` or `drone-N` |
| `HIVE_EXPOSED_PORTS` | Port mappings (e.g., `3000:13000,8081:18081`) |
| `HIVE_HOST_BASE` | Host address (`host.docker.internal`) |
| `REDIS_HOST` | Redis hostname |
| `REDIS_PASSWORD` | Redis auth password |

---

## Quick Reference

### Test a Web App
```
npm run dev &
hive_get_test_url(port=3000)
browser_navigate(url="...")
browser_screenshot()
```

### Test an Expo App
```
npx expo start &
hive_get_test_url(port=8081, protocol="exp")
ios_boot_device(deviceId="iPhone 15")
ios_open_url(deviceId="booted", url="...")
ios_screenshot(deviceId="booted")
```

### Read User's Clipboard
```
clipboard_read_text()
```

### Take Browser Screenshot
```
browser_navigate(url="...")
browser_screenshot()
```

### List Available MCPs
Use the built-in `/mcp` command to see all configured MCPs.
