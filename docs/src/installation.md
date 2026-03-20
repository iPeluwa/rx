# Installation

## One-liner

```sh
curl -fsSL https://raw.githubusercontent.com/iPeluwa/rx/master/install.sh | sh
```

Downloads a prebuilt binary for your platform (Linux, macOS, Windows/MSYS), or falls back to `cargo install` from source.

## From source

```sh
cargo install --path .
```

## GitHub Action

```yaml
- uses: iPeluwa/rx@v1
  with:
    command: ci
```

## Shell completions

```sh
rx completions bash >> ~/.bashrc
rx completions zsh >> ~/.zshrc
rx completions fish > ~/.config/fish/completions/rx.fish
rx completions powershell >> $PROFILE
```

Completions are context-aware — they include workspace members, installed targets, toolchains, and scripts.
