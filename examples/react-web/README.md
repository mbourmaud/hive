# React Web Example

A minimal React + Vite app demonstrating **autonomous browser testing** with Hive drones using Playwright.

## Quick Start

```bash
# 1. Copy this example to a new project
cp -r examples/react-web ~/my-react-app
cd ~/my-react-app

# 2. Start Hive (auto-setup on first run)
hive start

# 3. Connect to the Queen
hive connect queen
```

**That's it!** The `hive.yaml` is pre-configured. On first run, `hive start` automatically sets up everything.

## What Drones Can Do

With the Playwright MCP enabled, drones can autonomously:

| Action | Tool | Example |
|--------|------|---------|
| Navigate | `browser_navigate(url="...")` | Open any URL |
| Understand page | `browser_snapshot()` | Get accessibility tree |
| Click | `browser_click(element="...", ref="...")` | Click buttons/links |
| Type | `browser_type(element="...", text="...")` | Fill inputs |
| Wait | `browser_wait_for(text="...")` | Wait for content |
| Screenshot | `browser_screenshot()` | Capture evidence |

## Autonomous Testing Workflow

When a drone receives a task, it follows this workflow:

```
1. IMPLEMENT THE FEATURE
   - Edit src/pages/Products.tsx
   - Add new component

2. START DEV SERVER
   npm run dev &
   hive-log "SERVER RUNNING: http://localhost:5173"

3. GET TEST URL
   hive_get_test_url(port=5173)
   -> http://localhost:15173

4. NAVIGATE WITH PLAYWRIGHT
   browser_navigate(url="http://localhost:15173")
   browser_snapshot()  # Understand the page structure

5. INTERACT & TEST
   browser_click(element="Products link", ref="a[href='/products']")
   browser_wait_for(text="Products")
   browser_type(element="Search", ref="input.search-input", text="Widget")

6. CAPTURE RESULT
   browser_screenshot()

7. COMPLETE TASK
   hive_complete_task(result="Feature implemented. Tested with Playwright.")
```

## Project Structure

```
react-web/
├── src/
│   ├── main.tsx              # Entry point
│   ├── App.tsx               # Router
│   ├── pages/
│   │   ├── Home.tsx          # Landing page
│   │   ├── Products.tsx      # Product list with search
│   │   └── Contact.tsx       # Contact form
│   ├── components/
│   │   ├── Header.tsx        # Navigation
│   │   ├── Button.tsx        # Reusable button
│   │   └── Card.tsx          # Product card
│   └── styles/
│       └── globals.css       # Global styles
├── hive.yaml                 # Hive configuration
├── package.json              # Dependencies
└── vite.config.ts            # Vite config
```

## Configuration (hive.yaml)

```yaml
workspace:
  name: react-web-example

agents:
  queen:
    model: sonnet
  workers:
    count: 2
    dockerfile: docker/Dockerfile.node
    # Each drone gets its own port for Vite
    ports_per_drone:
      1: ["5173:15173"]
      2: ["5173:15174"]

host_mcps:
  playwright:
    enabled: true
    headless: false    # Watch tests in real browser!
  clipboard:
    enabled: true
```

### Port Mapping Explained

| Drone | Container Port | Host Port | Test URL |
|-------|----------------|-----------|----------|
| drone-1 | 5173 | 15173 | `http://localhost:15173` |
| drone-2 | 5173 | 15174 | `http://localhost:15174` |

## Example Tasks for Queen

### Add a New Feature

```bash
hive-assign drone-1 \
  "Add product detail modal" \
  "When clicking a product card on Products.tsx, show a modal with full details.
   Start dev server, test clicking a product, take screenshot of modal." \
  "WEB-001"
```

### Fix a Bug

```bash
hive-assign drone-2 \
  "Fix search not clearing" \
  "On Products.tsx, clicking X should clear the search input.
   Add a clear button if not present. Test the flow with Playwright." \
  "WEB-002"
```

### Add New Page

```bash
hive-assign drone-1 \
  "Add About page" \
  "Create src/pages/About.tsx with company info.
   Add link in Header.tsx. Test navigation flow." \
  "WEB-003"
```

## Playwright Selectors Reference

When using Playwright tools, you can reference elements by:

| Selector Type | Example | Use Case |
|---------------|---------|----------|
| Text content | `text="Products"` | Links, buttons |
| CSS selector | `ref="input.search-input"` | Specific elements |
| Role | `role=button[name="Submit"]` | Accessible elements |
| Test ID | `data-testid="submit-btn"` | Explicit test hooks |

## Troubleshooting

### Dev server not accessible

Make sure Vite is configured to listen on all interfaces:
```typescript
// vite.config.ts
server: {
  host: '0.0.0.0',
  port: 5173,
}
```

### Playwright can't find element

1. Use `browser_snapshot()` first to see the page structure
2. Check the accessibility tree for correct element names
3. Try different selector strategies (text, ref, role)

### Page not loading

1. Verify server is running: `curl http://localhost:5173`
2. Check port mapping in `hive.yaml`
3. Use `localhost`, not `localhost`

## Development Tips

1. **Always log server startup**: Use `hive-log "🚀 SERVER RUNNING: http://localhost:5173"` so Queen knows Vite is running.

2. **Use browser_snapshot() first**: Before interacting, always get the accessibility tree to understand the page.

3. **Take screenshots as evidence**: Always capture screenshots before marking tasks complete.

4. **Use headless: false for debugging**: Watch the browser as drones interact with it.

5. **Share via clipboard**: Use `clipboard_write_text()` to share URLs or data with the user.

## Skills Reference

Drones have access to step-by-step skill guides in `~/skills/`:

| Skill | Command | Description |
|-------|---------|-------------|
| Web Testing | `cat ~/skills/test-web.md` | Complete Playwright browser testing workflow |
| Port Networking | `cat ~/skills/port-networking.md` | Port mapping and URL discovery |

**Tip**: Before testing, drones should read the relevant skill to understand the workflow.

## Environment Variables

These are auto-configured by `hive init`:

| Variable | Purpose | Example |
|----------|---------|---------|
| `HIVE_EXPOSED_PORTS` | Port mappings | `5173:15173` |
| `HIVE_HOST_BASE` | Host address | `localhost` |

## Quick Reference

```bash
# Start project with Hive
cp -r examples/react-web ~/my-app && cd ~/my-app
hive start

# Connect to Queen
hive connect queen

# Assign web testing task
hive-assign drone-1 "Add dark mode" "Implement dark mode toggle in Header.tsx, test with Playwright" "TASK-1"

# Watch browser (headless: false)
# Browser opens automatically on your Mac when drones test!
```

## Testing Workflow Summary

```
1. npm run dev &                    # Start dev server
2. hive_get_test_url(port=5173)     # Get host-accessible URL
3. browser_navigate(url="...")       # Open in Playwright
4. browser_snapshot()               # Understand page structure
5. browser_click/type/wait          # Interact with elements
6. browser_screenshot()             # Capture evidence
7. hive_complete_task(result="...") # Mark as done
```
