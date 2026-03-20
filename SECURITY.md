# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability in rx, please report it responsibly:

1. **Do not** open a public GitHub issue.
2. Email the maintainer directly or use [GitHub's private vulnerability reporting](https://github.com/iPeluwa/rx/security/advisories/new).
3. Include a description of the vulnerability, steps to reproduce, and potential impact.

You should receive a response within 48 hours. We will work with you to understand and address the issue before any public disclosure.

## Scope

rx executes shell commands (`cargo`, `rustc`, `rustfmt`, etc.) on your behalf. It reads and writes files in your project directory and the `~/.rx` cache directory. Security concerns include:

- **Command injection** via malicious `rx.toml` config values
- **Path traversal** in cache operations
- **Dependency confusion** in registry operations
- **Checksum bypass** in self-update

We take these seriously and fix security issues with priority.
