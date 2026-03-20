# VS Code Extension

rx includes a VS Code extension located in the `editors/vscode/` directory of the repository. It provides editor integration for all major rx commands.

## Installation

Build and install the extension from source:

```sh
cd editors/vscode
npm install
npm run compile
# Then install the .vsix file, or use "Developer: Install Extension from Location"
```

The extension activates automatically when a workspace contains `Cargo.toml` or `rx.toml`.

## Commands

The extension provides 15 commands accessible from the Command Palette (Ctrl+Shift+P / Cmd+Shift+P):

| Command | Description |
|---------|-------------|
| `rx: Build` | Run `rx build` |
| `rx: Build (Release)` | Run `rx build --release` |
| `rx: Test` | Run `rx test` |
| `rx: Format` | Run `rx fmt` |
| `rx: Lint` | Run `rx lint` |
| `rx: Check` | Run `rx check` |
| `rx: Fix` | Run `rx fix` |
| `rx: Run CI` | Run `rx ci` |
| `rx: Clean` | Run `rx clean` |
| `rx: Doctor` | Run `rx doctor` |
| `rx: Insights` | Run `rx insights` |
| `rx: Dependencies` | Run `rx deps` |
| `rx: Coverage` | Run `rx coverage` |
| `rx: Watch` | Run `rx watch` |
| `rx: Run` | Run `rx run` |

## Task provider

The extension includes a VS Code task provider. Define tasks in `.vscode/tasks.json`:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "type": "rx",
      "command": "build --release",
      "label": "rx: Release Build"
    },
    {
      "type": "rx",
      "command": "test",
      "profile": "ci",
      "label": "rx: Test (CI profile)"
    }
  ]
}
```

Task properties:

| Property | Required | Description |
|----------|----------|-------------|
| `command` | Yes | The rx command to run |
| `profile` | No | Config profile to use |

## Auto-check on save

When `rx.autoCheck` is enabled (the default), the extension runs `rx check` automatically every time you save a Rust file. Errors and warnings appear in the Problems panel.

Disable it in VS Code settings:

```json
{
  "rx.autoCheck": false
}
```

## Problem matchers

The extension includes a problem matcher that parses Rust compiler errors and warnings from rx output. Errors appear as squiggly underlines in the editor and in the Problems panel, with clickable file/line references.

## Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `rx.path` | `"rx"` | Path to the rx binary. Set this if rx is not on your PATH. |
| `rx.autoCheck` | `true` | Run `rx check` automatically on save. |
| `rx.profile` | `""` | Default config profile. If set, all commands use this profile. |

## Tips

- Use `rx: Watch` to start continuous rebuilding in a terminal panel
- The `rx: Fix` command is useful as a keyboard shortcut for quick auto-fixing
- Set `rx.profile` to switch between development and CI configurations without editing `rx.toml`
- Task definitions support the `profile` property, so you can create tasks for different profiles
