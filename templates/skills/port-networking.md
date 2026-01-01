# Skill: Port Networking

Understand and manage container-to-host port mappings for testing.

## Overview

When running a dev server inside your container, you need to use the correct URL format for testing. This skill explains how port mapping works and how to get the right URLs.

## How Port Mapping Works

```
┌─────────────────┐                    ┌─────────────────┐
│   Container     │                    │     Host        │
│                 │                    │                 │
│  localhost:3000 │ ────────────────── │ localhost:13000 │
│  (Vite server)  │    Port Mapping    │                 │
│                 │                    │                 │
│  localhost:8081 │ ────────────────── │ localhost:18081 │
│  (Metro bundler)│                    │                 │
└─────────────────┘                    └─────────────────┘
```

**Key concept**: Your server runs on `localhost:CONTAINER_PORT` inside the container. From the host (Playwright, iOS Simulator), use `localhost:HOST_PORT`.

## Port Mapping in hive.yaml

```yaml
agents:
  workers:
    ports_per_drone:
      1: ["3000:13000", "8081:18081"]  # drone-1 ports
      2: ["3000:13001", "8081:18082"]  # drone-2 ports
```

Format: `CONTAINER_PORT:HOST_PORT`

## Getting Test URLs

### Method 1: MCP Tool (Recommended)

```
Use MCP tool: hive_get_test_url
Arguments: { "port": 3000 }
```

Returns: `http://localhost:13000`

For Expo apps:
```
Use MCP tool: hive_get_test_url
Arguments: { "port": 8081, "protocol": "exp" }
```

Returns: `exp://localhost:18081`

### Method 2: Environment Variable

```bash
echo $HIVE_EXPOSED_PORTS
# Output: 3000:13000,8081:18081
```

Parse it in your code or scripts.

### Method 3: List All Ports

```
Use MCP tool: hive_list_exposed_ports
```

Returns all port mappings with their URLs.

## Common Scenarios

### Web App (Vite/Next.js)

```bash
# Start server
npm run dev &  # Binds to localhost:5173

# Get test URL
hive_get_test_url(port=5173)  # → http://localhost:15173

# Use with Playwright
browser_navigate(url="http://localhost:15173")
```

### Expo/React Native

```bash
# Start Metro
npx expo start --port 8081 &

# Get test URL
hive_get_test_url(port=8081, protocol="exp")  # → exp://localhost:18081

# Use with iOS Simulator
ios_open_url(device="booted", url="exp://localhost:18081")
```

### API Server

```bash
# Start server
npm run server &  # Binds to localhost:4000

# Get test URL
hive_get_test_url(port=4000)  # → http://localhost:14000

# Test from host
curl http://localhost:14000/api/health
```

## URL Formats

| Scenario | URL Format |
|----------|------------|
| Inside container | `http://localhost:CONTAINER_PORT` |
| From host (HTTP) | `http://localhost:HOST_PORT` |
| Expo deep link | `exp://localhost:HOST_PORT` |

> **Remember**: Playwright and iOS Simulator run on the host, so use `localhost:HOST_PORT`.

## Troubleshooting

### "Connection refused"

1. Is your server running? Check: `pgrep -f "vite\|next\|expo"`
2. Is the port mapped? Check: `echo $HIVE_EXPOSED_PORTS`
3. Are you using the HOST port, not container port?

### "Wrong port in bundle URL"

For Expo, the Metro bundler needs to know about port mapping:
- Set `EXPO_PACKAGER_PROXY_URL=http://localhost:HOST_PORT` in .env

### Multiple Drones on Same Port

Each drone gets unique host ports:
- drone-1: `3000:13000`
- drone-2: `3000:13001`

Use `hive_get_test_url()` to get YOUR drone's specific port.

## Quick Reference

```
# Standard workflow
npm run dev &                            # Start server (container port)
hive_get_test_url(port=3000)             # Get host URL → http://localhost:13000
browser_navigate(url="http://localhost:13000")  # Test from host

# Environment check
echo $HIVE_EXPOSED_PORTS                 # See all mappings (e.g., 3000:13000,8081:18081)
```
