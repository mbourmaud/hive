# Skill: Test iOS App

Test your Expo/React Native app on iOS Simulator and capture screenshots.

## Prerequisites

- Metro bundler running (port 8081 or specified)
- iOS MCP enabled in hive.yaml
- Expo Go installed on simulator (auto-installed if needed)

## Workflow

### 1. Verify Server is Running

```bash
# Check if Metro is already running
pgrep -f "expo start" || {
  npx expo start --port 8081 &
  sleep 5
  hive-log "🚀 SERVER RUNNING: http://localhost:8081 (Metro bundler)"
}
```

### 2. Get Test URL

```
Use MCP tool: hive_get_test_url
Arguments: { "port": 8081, "protocol": "exp" }
```

Result: `exp://localhost:18081`

> **Note**: iOS Simulator runs on the host, so use `localhost:HOST_PORT`.

### 3. Prepare iOS Simulator

```
# List available devices
Use MCP tool: ios_list_devices

# Boot preferred device (iPhone 15 recommended)
Use MCP tool: ios_boot_device
Arguments: { "device": "iPhone 15" }

# Install Expo Go (automatic download)
Use MCP tool: ios_install_expo_go
Arguments: { "device": "booted" }
```

### 4. Open App in Expo Go

```
Use MCP tool: ios_open_url
Arguments: {
  "device": "booted",
  "url": "exp://localhost:18081"
}
```

Wait 5-10 seconds for the bundle to load.

### 5. Take Screenshot

```
Use MCP tool: ios_screenshot
Arguments: {
  "device": "booted",
  "outputPath": "/hive-shared/ios-test-result.png"
}
```

The screenshot is saved to `/hive-shared/` which is accessible from your container.

### 6. Verify Screenshot

Read the screenshot to verify the expected UI is displayed:

```
Read file: /hive-shared/ios-test-result.png
```

### 7. Navigate and Test Interactions

For interactive testing:

```
# Tap on specific coordinates
Use MCP tool: ios_tap
Arguments: { "device": "booted", "x": 200, "y": 400 }

# Type text in focused input
Use MCP tool: ios_type_text
Arguments: { "device": "booted", "text": "Hello World" }

# Take another screenshot
Use MCP tool: ios_screenshot
Arguments: { "device": "booted", "outputPath": "/hive-shared/ios-after-interaction.png" }
```

### 8. Report Results

```
Use MCP tool: hive_complete_task
Arguments: {
  "result": "iOS testing complete. Screenshots saved to /hive-shared/. App displays correctly on iPhone 15."
}
```

## Common Issues

### App Not Loading

1. Verify Metro bundler is running: `pgrep -f "expo start"`
2. Check port mapping in hive.yaml matches
3. Try restarting Metro: `npx expo start --clear --port 8081`

### Expo Go Not Installed

The `ios_install_expo_go` tool automatically downloads and installs Expo Go. If it fails:
1. Check internet connectivity
2. Try: `ios_get_status()` to verify Xcode is working
3. Manual install: Open App Store in simulator

### Wrong Port

Make sure your port mapping in hive.yaml matches:
```yaml
ports_per_drone:
  1: ["8081:18081"]  # container:host
```

The URL should use the HOST port (18081), not container port (8081).

## Quick Reference

```
# Full autonomous test sequence
npx expo start --port 8081 &
hive_get_test_url(port=8081, protocol="exp")  # → exp://localhost:18081
ios_boot_device(device="iPhone 15")
ios_install_expo_go(device="booted")
ios_open_url(device="booted", url="exp://localhost:18081")
ios_screenshot(device="booted", outputPath="/hive-shared/result.png")
hive_complete_task(result="Tested on iOS. Screenshot: /hive-shared/result.png")
```
