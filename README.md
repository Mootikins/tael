# tael

> **T**erminal-**A**gnostic **E**vent **L**ister

[![CI](https://github.com/moot/tael/actions/workflows/ci.yml/badge.svg)](https://github.com/moot/tael/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/tael.svg)](https://crates.io/crates/tael)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A lightweight TUI for tracking AI agent status across terminal panes. Know when your agents need attention without constantly checking each pane.

Named after [Tael](https://zelda.fandom.com/wiki/Tael), the purple fairy from Zelda: Majora's Mask.

## Features

- **Terminal-agnostic**: Works with Zellij, tmux, WezTerm, or any terminal
- **Interactive TUI**: Navigate with vim keys, press Enter to jump to pane
- **Lightweight**: Single Rust binary, no daemon required
- **Simple protocol**: Markdown-based persistence, easy to integrate

## Installation

```bash
cargo install tael
```

Or build from source:
```bash
git clone https://github.com/moot/tael
cd tael
cargo install --path .
```

## Usage

```bash
# Open interactive TUI (default)
tael

# Add an item
tael add "claude-code: Waiting for input" -p 42 --project myproject

# Add with git branch
tael add "claude-code: Review needed" -p 42 --project myproject -b feat/login

# List items
tael list

# Remove item
tael remove -p 42

# Clear all
tael clear

# Show config
tael config
```

### TUI Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Focus pane (jump to it) |
| `d` | Delete selected item |
| `r` | Reload inbox |
| `q` / `Esc` | Quit |

## Integration

### Claude Code Hooks

Add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "Notification": [
      {
        "matcher": "",
        "hooks": [
          "tael add \"$CLAUDE_NOTIFICATION\" -p \"$ZELLIJ_PANE_ID\" --project \"$(basename $PWD)\" -b \"$(git branch --show-current 2>/dev/null)\""
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          "tael remove -p \"$ZELLIJ_PANE_ID\""
        ]
      }
    ]
  }
}
```

### Zellij Keybinding

Add to your Zellij config to toggle the inbox with a hotkey:

```kdl
keybinds {
    shared {
        bind "Alt i" {
            Run "tael" {
                floating true
                close_on_exit true
            }
        }
    }
}
```

### tmux

For tmux, set the focus command in config or environment:

```bash
export TAEL_FOCUS_CMD="tmux select-pane -t {pane_id}"
```

Or in `~/.config/tael/config.toml`:

```toml
focus_command = "tmux select-pane -t {pane_id}"
```

## Configuration

Config file: `~/.config/tael/config.toml`

```toml
# Command to focus a pane. Use {pane_id} as placeholder.
# Auto-detected for Zellij and tmux if not set.
focus_command = "zellij action focus-pane-with-id {pane_id}"

# Checkbox style: "brackets", "circles", "bullets", or "none"
checkbox_style = "brackets"

# Enable ANSI colors
colors = true
```

## Environment Variables

| Variable | Description |
|----------|-------------|
| `TAEL_INBOX_FILE` | Override inbox file path |
| `TAEL_FOCUS_CMD` | Override focus command |
| `ZELLIJ_PANE_ID` | Auto-used for pane ID in Zellij |
| `ZELLIJ_SESSION_NAME` | Used for inbox file naming |

## How It Works

1. Agents (Claude Code, etc.) call `tael add` when they need attention
2. Agents call `tael remove` when they're done or user responds
3. You open `tael` TUI to see all waiting agents at a glance
4. Press Enter to jump directly to the pane that needs you

Inbox is stored as Markdown in `~/.local/share/tael/<session>.md`, making it easy to inspect or edit manually.

## License

MIT
