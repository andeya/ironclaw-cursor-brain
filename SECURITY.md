# Security

## Reporting vulnerabilities

If you believe you have found a security vulnerability, please report it responsibly:

- **Do not** open a public issue.
- Contact the maintainers (e.g. via the repository’s security contact or an issue asking for a private channel).
- Provide a clear description, steps to reproduce, and impact if possible.

We will acknowledge receipt and work with you to understand and address the issue.

## Scope

This project runs as a local HTTP service and spawns the Cursor Agent subprocess. Security-sensitive areas include:

- Configuration and file paths (e.g. `~/.ironclaw/`)
- Subprocess invocation and input/output handling
- Session persistence and any user-controlled data written to disk

We welcome reports related to these or other security concerns.
