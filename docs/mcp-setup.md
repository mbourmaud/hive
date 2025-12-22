# MCP (Model Context Protocol) Setup Guide

MCPs extend Claude's capabilities with external tools and APIs. With Hive's shared config, you configure MCPs **once** and they're available in all agents.

## How It Works

```
~/.claude/              # SHARED across all agents
├── mcp/                # ✅ MCP configurations
├── skills/             # ✅ Custom skills
└── claude.json         # ✅ Settings

Per-agent (isolated):
- history.jsonl         # Conversations
- session-env/          # Session state
```

## Quick Setup

### 1. Configure MCP on Host

```bash
# On your host machine (NOT in container)
claude

# Claude will prompt:
# "Would you like to set up Playwright MCP?"
# → Answer: yes

# Configure as prompted
```

### 2. Start Hive

```bash
cd ~/Projects/hive
hive start 3
```

### 3. MCP Available Everywhere ✅

```bash
# Connect to any agent
hive connect queen
# OR
hive connect 1

# MCP is automatically available!
```

## Common MCPs

### Playwright (Browser Automation)

**What it does:** Control Chrome/Firefox, take screenshots, scrape web pages

**Setup:**
```bash
claude  # Will prompt for Playwright setup
```

**Usage in Hive:**
```
# Tell Queen or any worker:
"Take a screenshot of https://example.com"
"Fill out the login form at https://app.com"
```

### Notion (Knowledge Base)

**What it does:** Read/write Notion pages and databases

**Prerequisites:**
- Notion API key
- Database IDs you want to access

**Setup:**
```bash
# Install Notion MCP
npm install -g @modelcontextprotocol/server-notion

# Configure in Claude
claude
# → Setup Notion MCP when prompted
# → Provide API key
```

**Usage:**
```
"Search Notion for project documentation"
"Create a new page in my tasks database"
```

### Jira (Issue Tracking)

**What it does:** Create/update Jira tickets, search issues

**Prerequisites:**
- Jira API token
- Jira server URL

**Setup:**
```bash
# Install Jira MCP
npm install -g @modelcontextprotocol/server-jira

# Configure
claude
# → Provide Jira URL and token
```

**Usage:**
```
"Create a bug ticket for the login timeout"
"List all open tickets assigned to me"
```

### GitHub (Code Repository)

**What it does:** Read repo files, create issues/PRs, search code

**Prerequisites:**
- GitHub personal access token

**Setup:**
```bash
# Install GitHub MCP
npm install -g @modelcontextprotocol/server-github

claude
# → Configure with your token
```

**Usage:**
```
"Search for TODO comments in the repo"
"Create a PR for branch feature/login"
```

## Custom MCPs

### Create Your Own MCP

```javascript
// my-custom-mcp/index.js
import { Server } from '@modelcontextprotocol/sdk/server/index.js';

const server = new Server({
  name: 'my-custom-mcp',
  version: '1.0.0'
});

server.setRequestHandler('tools/list', async () => ({
  tools: [
    {
      name: 'my_tool',
      description: 'What my tool does',
      inputSchema: {
        type: 'object',
        properties: {
          param: { type: 'string' }
        }
      }
    }
  ]
}));

server.setRequestHandler('tools/call', async (request) => {
  // Implement tool logic
  return { content: [{ type: 'text', text: 'Result' }] };
});

await server.connect(process.stdin, process.stdout);
```

### Install Custom MCP

```bash
# 1. Install your MCP
npm install -g ./my-custom-mcp

# 2. Configure in Claude
claude
# → Add MCP: my-custom-mcp
```

## Troubleshooting

### MCP Not Working in Container

**Symptom:** MCP works on host, but not in Hive agents

**Check:**
```bash
# 1. Verify config is shared
docker exec claude-queen ls -la ~/.claude/mcp
# Should show your MCP configs

# 2. Check Claude version
docker exec claude-queen claude --version
# Should be 2.0.75+

# 3. Restart Hive
hive stop
hive start 3
```

### Authentication Errors

**Symptom:** "API key invalid" or "Unauthorized"

**Solution:**
```bash
# Reconfigure MCP on host
claude

# Or manually edit config
vim ~/.claude/mcp/config.json
# Update tokens/API keys
```

### MCP Not Found

**Symptom:** "MCP server not found"

**Check installation:**
```bash
# On host
which playwright-mcp
npm list -g @modelcontextprotocol/server-playwright

# If missing, reinstall
npm install -g @modelcontextprotocol/server-playwright
```

## Per-Agent MCP Config (Advanced)

If you need **different MCPs per agent:**

```yaml
# docker-compose.override.yml
services:
  queen:
    volumes:
      # Queen gets full MCP access
      - ${HOME}/.claude:/home/agent/.claude

  agent-1:
    volumes:
      # Worker 1 only gets Playwright
      - ${HOME}/.claude/mcp/playwright.json:/home/agent/.claude/mcp/playwright.json:ro
      - shared-claude-minimal:/home/agent/.claude
```

## Resources

- [MCP Documentation](https://modelcontextprotocol.io/)
- [Available MCP Servers](https://github.com/modelcontextprotocol/servers)
- [Build Your Own MCP](https://modelcontextprotocol.io/docs/guides/building-server)

## Summary

✅ **Configure once on host** → Available everywhere
✅ **Shared across Queen + Workers** → No duplication
✅ **Persists between restarts** → No re-setup
✅ **Isolated conversations** → Each agent keeps own history

MCPs make Hive agents incredibly powerful! 🚀
