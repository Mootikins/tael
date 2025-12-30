<div align="center">
<table><tr><td valign="middle">
<img src="assets/mascot.png" alt="tael mascot" width="140">
</td><td valign="middle">
<h1>tael</h1>
<strong>T</strong>erminal-<strong>A</strong>gnostic <strong>E</strong>vent <strong>L</strong>ister<br><br>
<a href="https://github.com/Mootikins/tael/actions/workflows/ci.yml"><img src="https://github.com/Mootikins/tael/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
<a href="https://crates.io/crates/tael"><img src="https://img.shields.io/crates/v/tael.svg" alt="Crates.io"></a>
<a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT"></a>
</td></tr></table>
</div>

<p align="center">
  A lightweight TUI for tracking AI agent status across terminal panes.<br>
  Know when your agents need attention without constantly checking each pane.
</p>

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
git clone https://github.com/Mootikins/tael
cd tael
cargo install --path .
```

## Usage

```bash
# Open interactive TUI (default)
tael

# Add an item with attributes
tael add -a "msg=claude-code: Waiting for input" -a pane=42 -a proj=myproject

# Add with JSON stdin (extract fields with @.field syntax)
echo '{"message":"Auth needed"}' | tael add -a "msg=@.message" -a pane=42

# Claude Code preset (extracts message/type from JSON stdin)
echo "$NOTIFICATION_JSON" | tael add --from-claude-code -a pane=$PANE_ID

# List items (with optional grouping)
tael list
tael list --group-by proj
tael list --group-by status,proj

# Remove item by pane
tael remove -a pane=42

# Clear all
tael clear

# Launch TUI in a floating pane (Zellij only)
tael float
tael float -p bottom-left --width 40% --height 60%
```

### TUI Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Enter` | Focus pane (jump to it) |
| `d` | Delete selected item |
| `p` | Pin floating pane (Zellij only) |
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
          {
            "type": "command",
            "command": "tael add --from-claude-code -a pane=$ZELLIJ_PANE_ID -a proj=$(basename $PWD) -a branch=$(git branch --show-current 2>/dev/null)"
          }
        ]
      }
    ],
    "Stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "tael remove -a pane=$ZELLIJ_PANE_ID"
          }
        ]
      }
    ]
  }
}
```

The `--from-claude-code` flag reads JSON from stdin and extracts `message` and `notification_type` fields automatically.

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

For tmux, set the focus command via environment or CLI flag:

```bash
export TAEL_FOCUS_CMD="tmux select-pane -t {pane_id}"
```

Or pass directly:

```bash
tael --focus-cmd "tmux select-pane -t {pane_id}"
```

## Configuration

tael is configured entirely via CLI flags and environment variables (no config files).

| Flag | Env Variable | Description |
|------|--------------|-------------|
| `--focus-cmd` | `TAEL_FOCUS_CMD` | Command to focus a pane (use `{pane_id}` placeholder) |
| `-f, --file` | `TAEL_INBOX_FILE` | Override inbox file path |
| `--group-by` | - | Group items by attribute (e.g., `status,proj`) |

Focus command is auto-detected for Zellij and tmux if not specified.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ZELLIJ_PANE_ID` | Auto-used for pane ID in Zellij hooks |
| `ZELLIJ_SESSION_NAME` | Used for per-session inbox file naming |
| `TMUX` | Detected for tmux focus command auto-config |

## How It Works

1. Agents (Claude Code, etc.) call `tael add` when they need attention
2. Agents call `tael remove` when they're done or user responds
3. You open `tael` TUI to see all waiting agents at a glance
4. Press Enter to jump directly to the pane that needs you

Inbox is stored as Markdown in `~/.local/share/tael/<session>.md`, making it easy to inspect or edit manually.

## License

MIT
