---
name: ralph-loop
description: Execute tasks using Ralph Loop pattern - iterate until verified complete
allowed-tools:
  - Task
  - Bash
  - Read
  - Write
  - Edit
  - Glob
  - Grep
user-invocable: true
---

# Ralph Loop Task Execution

You are executing a task using the **Ralph Loop** pattern. This means you will iterate continuously until the task is VERIFIED complete.

## Pattern

```
while not verified:
    execute()
    result = verify()
    if result.failed:
        analyze_and_fix()
        iteration++
    else:
        commit()
        done()
```

## Execution Rules

1. **NEVER stop on first attempt** - always verify
2. **Use sub-agents for parallelization** when task spans multiple layers
3. **Run `hive-verify`** before marking complete
4. **Commit atomically** - one logical change per commit

## Sub-Agent Dispatch

For full-stack tasks, spawn parallel sub-agents:

```
Task("contract", "Create ts-rest contract for [endpoint]")
Task("gateway", "Implement NestJS resolver for [endpoint]")  
Task("frontend", "Create React hook/component for [feature]")
Task("tests", "Write integration tests for [feature]")
```

## Verification Checklist

Before completing, ensure:
- [ ] `npm run typecheck` passes (or `go vet`)
- [ ] `npm run test` passes (or `go test`)
- [ ] `npm run build` passes (or `go build`)
- [ ] Code is clean and documented
- [ ] Changes are committed

## When to Escalate

After 3 failed iterations on the same issue:
```bash
hive-solicit '{
  "type": "blocker",
  "urgency": "high",
  "message": "[describe the issue]",
  "iterations": 3
}'
```

## Completion

Only after ALL verifications pass:
```bash
hive-complete '{"result": "[summary of what was done]"}'
```
