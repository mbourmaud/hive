# Hive v3 - Claude Agent SDK Migration Plan

**Status:** 📋 PLANNED (Not Started)  
**Priority:** Medium (Future Enhancement)  
**Estimated Effort:** 2-3 weeks  
**Created:** 2026-01-08

---

## Executive Summary

Migrate Hive from **AgentAPI + Claude CLI** to **Claude Agent SDK** to gain:

| Feature | Current (v2) | Target (v3) |
|---------|--------------|-------------|
| Agent Loop | AgentAPI HTTP wrapper | SDK `query()` native |
| Subagents | Custom `Task()` via CLAUDE.md | Native `agents` param |
| Hooks | None (prompt-based) | `PreToolUse`, `PostToolUse`, etc. |
| Output | Text parsing | JSON Schema structured |
| Cost Tracking | Manual | Built-in `total_cost_usd` |
| Sessions | Implicit (agentapi state) | Explicit `resume: sessionId` |
| Permissions | `--dangerously-skip-permissions` | `canUseTool()` callbacks |

---

## Architecture Comparison

### Current (v2)
```
Hub (Go) → HTTP → AgentAPI → claude CLI → worktree
```

### Target (v3) - Option A: Hybrid (Recommended)
```
Hub (Go) → IPC → Node SDK Process → Claude SDK → worktree
```

### Target (v3) - Option B: Pure Node.js
```
Hub (Node.js) → query() → Claude SDK → worktree
```

---

## Phase 1: Prototype (Week 1)

### Goal
Prove SDK can replace AgentAPI for a single drone.

### Deliverables

1. **`packages/hive-drone-sdk/`** - Node.js package
   ```typescript
   // src/index.ts
   import { query } from "@anthropic-ai/claude-agent-sdk";
   
   interface DroneTask {
     prompt: string;
     worktreePath: string;
     agentId: string;
     hubUrl: string;
   }
   
   async function runDrone(task: DroneTask) {
     for await (const msg of query({
       prompt: task.prompt,
       options: {
         cwd: task.worktreePath,
         allowedTools: ["Read", "Write", "Edit", "Bash", "Glob", "Grep", "Task"],
         permissionMode: "bypassPermissions",
         settingSources: ["project"],  // Load CLAUDE.md
         systemPrompt: { type: "preset", preset: "claude_code" }
       }
     })) {
       // Output JSON lines for Go hub to consume
       console.log(JSON.stringify({
         agentId: task.agentId,
         message: msg
       }));
     }
   }
   
   // Read task from stdin
   const input = await readStdin();
   await runDrone(JSON.parse(input));
   ```

2. **Test harness** - Standalone test
   ```bash
   echo '{"prompt":"List files","worktreePath":"/tmp/test"}' | npx hive-drone-sdk
   ```

3. **Validation checklist**:
   - [ ] SDK respects `cwd` for file operations
   - [ ] CLAUDE.md is loaded from worktree
   - [ ] Messages stream correctly
   - [ ] Subagents work
   - [ ] Cost is reported

---

## Phase 2: Feature Parity (Week 2)

### Goal
Match v2 Hive features using SDK.

### Deliverables

1. **Hooks implementation**
   ```typescript
   const auditHook: HookCallback = async (input, toolUseId, { signal }) => {
     await fetch(`${hubUrl}/agents/${agentId}/audit`, {
       method: "POST",
       body: JSON.stringify({
         event: input.hook_event_name,
         tool: input.tool_name,
         input: input.tool_input
       })
     });
     return {};
   };
   
   const securityHook: HookCallback = async (input) => {
     if (input.hook_event_name === "PreToolUse" && input.tool_name === "Bash") {
       const cmd = input.tool_input.command;
       if (cmd.includes("rm -rf /") || cmd.includes("sudo")) {
         return {
           hookSpecificOutput: {
             hookEventName: "PreToolUse",
             permissionDecision: "deny",
             permissionDecisionReason: "Dangerous command blocked"
           }
         };
       }
     }
     return {};
   };
   ```

2. **Structured output for task results**
   ```typescript
   const taskResultSchema = {
     type: "object",
     properties: {
       status: { type: "string", enum: ["success", "failed", "needs_input"] },
       summary: { type: "string" },
       files_changed: { type: "array", items: { type: "string" } },
       tests_passed: { type: "boolean" },
       build_passed: { type: "boolean" },
       cost_usd: { type: "number" }
     },
     required: ["status", "summary"]
   };
   ```

3. **Native subagents** (replace Task() in CLAUDE.md)
   ```typescript
   agents: {
     "contract": {
       description: "Creates ts-rest API contracts",
       prompt: "You specialize in ts-rest contracts...",
       tools: ["Read", "Write", "Edit"],
       model: "sonnet"
     },
     "frontend": {
       description: "React/Next.js frontend development",
       prompt: "You build React components...",
       tools: ["Read", "Write", "Edit", "Bash"],
       model: "sonnet"
     },
     "tests": {
       description: "Integration test writer",
       prompt: "You write comprehensive tests...",
       tools: ["Read", "Write", "Bash"],
       model: "haiku"
     }
   }
   ```

4. **Ralph Loop integration**
   - Port `ralph-loop.md` skill logic to SDK hooks
   - Implement `hive-verify` as PostToolUse hook

---

## Phase 3: Hub Integration (Week 2-3)

### Goal
Integrate SDK drone with Go Hub.

### Option A: IPC (Recommended)

```go
// internal/agent/sdk_spawner.go
func (s *SDKSpawner) Spawn(ctx context.Context, opts SpawnOptions) (*Agent, error) {
    // Create worktree (same as before)
    wt, err := s.worktreeMgr.Create(ctx, ...)
    
    // Spawn Node.js SDK process instead of agentapi
    cmd := exec.Command("npx", "hive-drone-sdk")
    cmd.Dir = wt.Path
    
    stdin, _ := cmd.StdinPipe()
    stdout, _ := cmd.StdoutPipe()
    
    cmd.Start()
    
    // Send initial config
    json.NewEncoder(stdin).Encode(DroneConfig{
        AgentID: id,
        HubURL: hubURL,
        // ...
    })
    
    // Start message reader goroutine
    go s.readMessages(agent, stdout)
    
    return agent, nil
}

func (s *SDKSpawner) readMessages(agent *Agent, stdout io.Reader) {
    scanner := bufio.NewScanner(stdout)
    for scanner.Scan() {
        var msg SDKMessage
        json.Unmarshal(scanner.Bytes(), &msg)
        s.hub.Broadcast(agent.ID, msg)
    }
}
```

### Option B: Full Node.js Rewrite

Rewrite Hub in TypeScript. Larger effort but cleaner architecture.

---

## Phase 4: Deprecation (Week 3+)

### Goal
Remove AgentAPI dependency.

### Steps

1. **Feature flag**: `HIVE_USE_SDK=1`
2. **Parallel operation**: Both spawners available
3. **Testing**: Run integration tests with SDK spawner
4. **Migration**: Default to SDK spawner
5. **Removal**: Delete AgentAPI code

---

## Files to Create/Modify

### New Files
```
packages/
└── hive-drone-sdk/
    ├── package.json
    ├── tsconfig.json
    └── src/
        ├── index.ts        # Main entry
        ├── hooks.ts        # Hook implementations
        ├── agents.ts       # Subagent definitions
        └── types.ts        # TypeScript types
```

### Modified Files
```
internal/agent/
├── spawner.go          # Add SDKSpawner
├── sdk_spawner.go      # New SDK-based spawner
└── types.go            # Add SDK message types

cmd/
└── spawn.go            # Add --use-sdk flag
```

### Removed Files (Phase 4)
```
- AgentAPI dependency
- internal/agent/hive-commands.sh (move to SDK)
```

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| SDK bugs | High | Keep AgentAPI as fallback |
| Node.js dependency | Medium | Bundle with Hive release |
| Performance overhead | Low | SDK is efficient |
| Subagent depth limit | Medium | SDK subagents can't spawn subagents - restructure Ralph Loop |

---

## Success Criteria

- [ ] Single drone works with SDK
- [ ] Hooks capture all tool usage
- [ ] Structured output parses correctly
- [ ] Cost tracking accurate
- [ ] Ralph Loop works with native subagents
- [ ] No regression in existing tests
- [ ] AgentAPI fully removed

---

## References

- [Claude Agent SDK Overview](https://platform.claude.com/docs/en/agent-sdk/overview)
- [TypeScript SDK Reference](https://platform.claude.com/docs/en/agent-sdk/typescript)
- [Subagents Documentation](https://platform.claude.com/docs/en/agent-sdk/subagents)
- [Hooks Documentation](https://platform.claude.com/docs/en/agent-sdk/hooks)
- [@dabit3's SDK Tutorial](https://x.com/dabit3/status/2009131298250428923)
- [@godofprompt's Skills Deep Dive](https://x.com/godofprompt/status/2008578110141190580)

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-01-08 | Plan created | SDK offers significant advantages over AgentAPI |
| 2026-01-08 | Deferred to v3 | Focus on using Hive in real projects first |
| TBD | Choose Option A or B | Pending prototype results |
