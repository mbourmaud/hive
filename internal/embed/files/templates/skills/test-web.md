# Skill: Test Web App

Test your web application with Playwright browser automation and capture screenshots.

## Prerequisites

- Dev server running (Vite, Next.js, etc.)
- Playwright MCP enabled in hive.yaml
- Browser will be controlled on host machine

## Workflow

### 1. Verify Server is Running

```bash
# For Vite (React/Vue)
pgrep -f "vite" || {
  npm run dev &
  sleep 3
  hive-log "🚀 SERVER RUNNING: http://localhost:5173 (Vite dev server)"
}

# For Next.js
pgrep -f "next" || {
  npm run dev &
  sleep 3
  hive-log "🚀 SERVER RUNNING: http://localhost:3000 (Next.js dev server)"
}
```

### 2. Get Test URL

```
Use MCP tool: hive_get_test_url
Arguments: { "port": 5173 }
```

Result: `http://localhost:15173`

> **Note**: Playwright runs on the host, so use `localhost:HOST_PORT`.

### 3. Navigate to App

```
Use MCP tool: browser_navigate
Arguments: { "url": "http://localhost:15173" }
```

### 4. Understand Page Structure

Before interacting, always capture the accessibility tree:

```
Use MCP tool: browser_snapshot
```

This returns the page structure with element references you can use for clicking/typing.

### 5. Take Screenshot

```
Use MCP tool: browser_screenshot
```

The screenshot shows what the user would see.

### 6. Interact with Elements

Based on the snapshot, interact with elements:

```
# Type in an input
Use MCP tool: browser_type
Arguments: {
  "element": "search input",
  "ref": "input[name='search']",
  "text": "hello world"
}

# Click a button
Use MCP tool: browser_click
Arguments: {
  "element": "submit button",
  "ref": "button[type='submit']"
}

# Wait for content to appear
Use MCP tool: browser_wait_for
Arguments: { "text": "Results found" }
```

### 7. Navigate Through App

```
# Click navigation links
Use MCP tool: browser_click
Arguments: { "element": "Products link", "ref": "a[href='/products']" }

# Wait for page load
Use MCP tool: browser_wait_for
Arguments: { "text": "Product List" }

# Take screenshot of new page
Use MCP tool: browser_screenshot
```

### 8. Fill Forms

```
Use MCP tool: browser_fill_form
Arguments: {
  "fields": [
    { "ref": "input[name='email']", "value": "test@example.com" },
    { "ref": "input[name='password']", "value": "secret123" },
    { "ref": "input[name='name']", "value": "Test User" }
  ]
}
```

### 9. Check Console for Errors

```
Use MCP tool: browser_console_messages
```

Verify there are no JavaScript errors.

### 10. Report Results

```
Use MCP tool: hive_complete_task
Arguments: {
  "result": "Web testing complete. Verified: login flow, navigation, form submission. No console errors."
}
```

## Common Test Scenarios

### Login Flow
```
browser_navigate(url="http://localhost:15173/login")
browser_snapshot()
browser_type(element="email", ref="input[name='email']", text="user@test.com")
browser_type(element="password", ref="input[name='password']", text="password123")
browser_click(element="login button", ref="button[type='submit']")
browser_wait_for(text="Welcome")
browser_screenshot()
```

### Form Validation
```
browser_navigate(url="http://localhost:15173/contact")
browser_click(element="submit button", ref="button[type='submit']")  # Empty submit
browser_snapshot()  # Should show validation errors
browser_screenshot()
```

### Navigation Test
```
browser_navigate(url="http://localhost:15173")
browser_click(element="Products", ref="a[href='/products']")
browser_wait_for(text="Product")
browser_click(element="first product", ref=".product-card:first-child")
browser_wait_for(text="Details")
browser_screenshot()
```

## Common Issues

### Element Not Found

1. Run `browser_snapshot()` first to see available elements
2. Use more specific selectors (id, data-testid, etc.)
3. Wait for element: `browser_wait_for(text="...")`

### Wrong Port

Check your port mapping in hive.yaml:
```yaml
ports_per_drone:
  1: ["5173:15173"]  # container:host
```

Use the HOST port (15173) in the URL.

### Page Not Loading

1. Verify dev server is running: `pgrep -f "vite"`
2. Check the test URL is correct: `hive_get_test_url(port=5173)`
3. Try `browser_navigate` with the full URL

## Quick Reference

```
# Full autonomous test sequence
npm run dev &
hive_get_test_url(port=5173)  # → http://localhost:15173
browser_navigate(url="http://localhost:15173")
browser_snapshot()            # Understand page
browser_click(element="...", ref="...")
browser_screenshot()          # Capture evidence
hive_complete_task(result="Web testing complete")
```

## Advanced: Visual Regression

For comparing screenshots:

```
# Before changes
browser_screenshot()  # Save this as baseline

# After changes
browser_screenshot()  # Compare with baseline
```

## Advanced: Network Inspection

```
# View network requests
Use MCP tool: browser_network_requests

# Check API calls are working
# Verify status codes, response data
```
