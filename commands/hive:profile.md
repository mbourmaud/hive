# Hive Profile - Manage Claude Profiles

Manage Claude profiles for different Claude wrappers or API configurations (Bedrock, MAX, custom wrappers, etc.).

## Usage

- `/hive:profile` - List all profiles
- `/hive:profile list` - List all profiles (alias)
- `/hive:profile add <name> <command> [description]` - Add a new profile
- `/hive:profile set-default <name>` - Set default profile
- `/hive:profile rm <name>` - Remove a profile

## What are Profiles?

Profiles allow you to configure different Claude commands for hive drones:
- **Default**: Standard `claude` CLI
- **Custom wrappers**: `claude-wrapper ml` (Bedrock), `claude-wrapper perso` (MAX API)
- **Direct API calls**: Any custom command that wraps Claude

Profiles are stored in `~/.config/hive/config.json`.

## Arguments

Parse arguments from the command if provided:

| Argument | Description | Example |
|----------|-------------|---------|
| `<name>` | Profile name | `ml`, `perso`, `bedrock` |
| `<command>` | Claude command to execute | `claude-wrapper ml`, `claude` |
| `[description]` | Optional description | `"Bedrock (work)"` |

**Examples:**
```
/hive:profile
/hive:profile list
/hive:profile add ml "claude-wrapper ml" "Bedrock (work)"
/hive:profile add perso "claude-wrapper perso" "MAX API (personal)"
/hive:profile set-default ml
/hive:profile rm perso
```

## Workflow

### For `/hive:profile` or `/hive:profile list`

1. Run `hive profile list` via Bash tool
2. Display the output (formatted list of profiles with default marked)
3. Show example usage: `Use 'hive start <prd> --profile <name>' to use a specific profile`

### For `/hive:profile add <name> <command> [description]`

**IMPORTANT: Extract arguments carefully from the command.**

1. Parse the command arguments:
   - `<name>`: First argument after 'add' (required)
   - `<command>`: Second argument (required) - MUST be quoted if it contains spaces
   - `[description]`: Third argument (optional)

2. Validate:
   - Name must not be empty
   - Command must not be empty
   - Command should be a valid executable or wrapper

3. Run: `hive profile add "<name>" "<command>" "<description>"`
   - Use proper quoting for bash command
   - Example: `hive profile add ml "claude-wrapper ml" "Bedrock (work)"`

4. Confirm success and show updated profile list

**Example parsing:**
```
User: /hive:profile add ml "claude-wrapper ml" "Bedrock (work)"
→ name="ml"
→ command="claude-wrapper ml"
→ description="Bedrock (work)"
→ Execute: hive profile add ml "claude-wrapper ml" "Bedrock (work)"
```

### For `/hive:profile set-default <name>`

1. Parse `<name>` from command arguments
2. Run: `hive profile set-default "<name>"`
3. Confirm the default profile was changed
4. Show current profiles with new default marked

### For `/hive:profile rm <name>`

1. Parse `<name>` from command arguments
2. Warn if removing non-default profile (just informational)
3. Run: `hive profile rm "<name>"`
4. Confirm removal
5. Note: Cannot remove 'default' profile (hive will error)

## Common Use Cases

### User wants to use Bedrock for work
```
User: "I want to use my claude-wrapper ml for hive drones"
→ /hive:profile add ml "claude-wrapper ml" "Bedrock (work)"
→ /hive:profile set-default ml
→ Confirm: "Default profile set to 'ml'. All future drones will use 'claude-wrapper ml'"
```

### User wants to switch between profiles
```
User: "Set perso as default for now"
→ /hive:profile set-default perso
→ Explain: "Now hive start will use 'claude-wrapper perso' by default. Use --profile ml to override."
```

### User asks what profiles exist
```
User: "What profiles do I have?"
→ /hive:profile list
→ Display formatted output
```

## Error Handling

- **Profile not found**: Show available profiles and suggest correct name
- **Missing arguments**: Show usage example with proper syntax
- **Invalid command**: Warn if the command doesn't look like a valid executable

## Integration with hive start

After setting up profiles, users can:
- Use default: `hive start my-prd`
- Override: `hive start my-prd --profile perso`

**Explain this workflow when users set up profiles for the first time.**

## Tips

- Profile names should be short and memorable (ml, perso, bedrock)
- Commands with spaces MUST be quoted: `"claude-wrapper ml"`
- Descriptions help identify profiles: `"Bedrock (work)"`, `"MAX API (personal)"`
- The 'default' profile cannot be removed, only modified
