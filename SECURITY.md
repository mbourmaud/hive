# Security Policy

## Supported Versions

We release patches for security vulnerabilities in the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 3.0.x   | :white_check_mark: |
| < 3.0   | :x:                |

## Reporting Vulnerabilities

We take the security of Hive seriously. If you discover a security vulnerability, please follow these steps:

### Reporting Process

1. **DO NOT** open a public issue for security vulnerabilities
2. Report the vulnerability by opening a private security advisory on our [GitHub repository](https://github.com/mbourmaud/hive/security/advisories/new)
3. Alternatively, create a regular issue at [GitHub Issues](https://github.com/mbourmaud/hive/issues) if the vulnerability is not critical

### What to Include

Please include the following information in your report:

- A description of the vulnerability
- Steps to reproduce the issue
- Potential impact of the vulnerability
- Any possible mitigations you've identified
- Your contact information (optional)

### Response Timeline

- We will acknowledge receipt of your vulnerability report within 48 hours
- We will provide a detailed response within 7 days, including next steps
- We will keep you informed of the progress toward a fix
- Once the vulnerability is resolved, we will publicly disclose it (with credit to you, if desired)

## Security Best Practices

When using Hive, we recommend the following security practices:

### Credential Management

- Never commit credentials or API keys to your repository
- Use environment variables or secure credential storage for sensitive data
- Ensure your Claude API key is properly secured and not exposed in logs

### Git and GitHub Security

- Review all automated commits and PRs before merging
- Use branch protection rules for important branches
- Enable two-factor authentication for your GitHub account
- Regularly audit drone activities using `hive logs` and `hive monitor`

### Worktree Isolation

- Each drone operates in an isolated git worktree
- Regularly clean up unused drones with `hive clean`
- Monitor drone activities to ensure no unexpected behavior

### CLI Security

- Keep Hive updated to the latest version using `hive update`
- Verify the integrity of the installation script before running
- Review permissions required by the CLI and associated tools (git, gh, claude)

## Dependencies

Hive relies on the following external tools. Ensure these are kept up to date:

- `git` - Version control system
- `gh` - GitHub CLI
- `claude` - Claude Code CLI

Vulnerabilities in these dependencies may affect Hive's security. Please keep them updated to their latest stable versions.

## Scope

This security policy applies to:

- The Hive CLI tool (`hive` binary)
- The installation script
- Documentation and configuration files

It does not cover:

- Third-party dependencies (report those to the respective projects)
- Issues with the underlying Claude Code, git, or gh CLI tools
- User-generated plans or custom configurations

## Questions

If you have questions about this security policy, please open an issue at [GitHub Issues](https://github.com/mbourmaud/hive/issues).
