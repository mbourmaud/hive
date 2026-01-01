# Expo Mobile Example

A minimal Expo/React Native app demonstrating **autonomous iOS Simulator testing** with Hive drones.

## Quick Start

```bash
# 1. Copy this example to a new project
cp -r examples/expo-mobile ~/my-expo-app
cd ~/my-expo-app

# 2. Start Hive (auto-setup on first run)
hive start

# 3. Connect to the Queen
hive connect queen
```

**That's it!** The `hive.yaml` is pre-configured. On first run, `hive start` automatically sets up everything.

## What Drones Can Do

With the iOS MCP enabled, drones can autonomously:

| Action | Tool | Example |
|--------|------|---------|
| List simulators | `ios_list_devices()` | Find available iPhones |
| Boot simulator | `ios_boot_device(device="iPhone 15")` | Start the simulator |
| Install Expo Go | `ios_install_expo_go(device="iPhone 15")` | Auto-install Expo Go |
| Open app | `ios_open_url(device="booted", url="exp://...")` | Launch in Expo Go |
| Take screenshot | `ios_screenshot(device="booted", outputPath="...")` | Capture evidence |

## Autonomous Testing Workflow

When a drone receives a task, it follows this workflow:

```
1. IMPLEMENT THE FEATURE
   - Edit app/(tabs)/list.tsx
   - Add new component

2. START METRO BUNDLER
   npx expo start --port 8081 &
   hive-log "SERVER RUNNING: http://localhost:8081"

3. GET TEST URL
   hive_get_test_url(port=8081, protocol="exp")
   -> exp://host.docker.internal:18081

4. PREPARE SIMULATOR
   ios_list_devices()
   ios_boot_device(device="iPhone 15")
   ios_install_expo_go(device="iPhone 15")

5. OPEN APP IN EXPO GO
   ios_open_url(device="booted", url="exp://host.docker.internal:18081")

6. TEST & CAPTURE
   ios_screenshot(device="booted", outputPath="/hive-shared/done.png")

7. COMPLETE TASK
   hive_complete_task(result="Feature implemented. See /hive-shared/done.png")
```

## Project Structure

```
expo-mobile/
├── app/
│   ├── _layout.tsx          # Tab navigation
│   └── (tabs)/
│       ├── index.tsx        # Home screen
│       ├── list.tsx         # Item list
│       └── profile.tsx      # User profile
├── components/
│   ├── Button.tsx           # Reusable button
│   └── Card.tsx             # List item card
├── hive.yaml                # Hive configuration
├── package.json             # Dependencies
└── app.json                 # Expo config
```

## Configuration (hive.yaml)

```yaml
workspace:
  name: expo-mobile-example

agents:
  queen:
    model: sonnet
  workers:
    count: 2
    dockerfile: docker/Dockerfile.node
    # Each drone gets its own port for Metro
    ports_per_drone:
      1: ["8081:18081"]
      2: ["8081:18082"]

host_mcps:
  ios:
    enabled: true    # Required for iOS Simulator
  playwright:
    enabled: true    # Optional: for Expo Web testing
  clipboard:
    enabled: true
```

### Port Mapping Explained

| Drone | Container Port | Host Port | Test URL |
|-------|----------------|-----------|----------|
| drone-1 | 8081 | 18081 | `exp://host.docker.internal:18081` |
| drone-2 | 8081 | 18082 | `exp://host.docker.internal:18082` |

## Example Tasks for Queen

### Add a New Feature

```bash
hive-assign drone-1 \
  "Add search bar to item list" \
  "Add a search input at the top of list.tsx that filters items by title.
   Start Metro, test on iOS Simulator, take screenshot." \
  "MOBILE-001"
```

### Fix a Bug

```bash
hive-assign drone-2 \
  "Fix profile stats alignment" \
  "The stats on profile.tsx are not evenly spaced.
   Fix the flexbox layout. Test on iPhone 15, capture before/after screenshots." \
  "MOBILE-002"
```

### Add New Screen

```bash
hive-assign drone-1 \
  "Add item detail screen" \
  "Create app/item/[id].tsx that shows full item details.
   Navigate from list.tsx when tapping a card. Test navigation flow." \
  "MOBILE-003"
```

## Screenshots Directory

Screenshots taken by drones are saved to `/hive-shared/` inside containers, which maps to `.hive/shared/` on your host machine.

```bash
# View screenshots
ls .hive/shared/
open .hive/shared/feature-done.png
```

## Troubleshooting

### Metro bundler not accessible

Make sure the port mapping is correct in `hive.yaml`:
```yaml
ports_per_drone:
  1: ["8081:18081"]
```

### Expo Go not installed

Drones can auto-install it:
```
ios_install_expo_go(device="iPhone 15")
```

### Simulator not booting

Check available simulators:
```
ios_list_devices()
ios_get_status()
```

### App not loading in Expo Go

1. Verify Metro is running: `curl http://localhost:8081`
2. Check the test URL: `hive_get_test_url(port=8081, protocol="exp")`
3. Ensure you're using `host.docker.internal`, not `localhost`

## Development Tips

1. **Always log server startup**: Use `hive-log "SERVER RUNNING: http://localhost:8081"` so Queen knows Metro is running.

2. **Use screenshots as evidence**: Always capture screenshots before marking tasks complete.

3. **Test on multiple devices**: Use `ios_list_devices()` to find different iPhone models.

4. **Share data via clipboard**: Use `clipboard_write_text()` to share URLs or data with the user.
