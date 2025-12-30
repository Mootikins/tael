# AI Agent Guide for tael

> Instructions for AI agents (Claude, Codex, etc.) working on the tael codebase

## Project Overview

**tael** (**T**erminal-**A**gnostic **E**vent **L**ister) is a lightweight TUI for tracking AI agent status across terminal panes. Named after [Tael](https://zelda.fandom.com/wiki/Tael), the purple fairy from Zelda: Majora's Mask.

## Architecture

Single-binary Rust CLI with these modules:

| Module | Purpose |
|--------|---------|
| `main.rs` | CLI entry point, clap-based commands, JSON extraction |
| `types.rs` | `Inbox`, `InboxItem`, `Status` types |
| `parse.rs` | Markdown → Inbox parsing |
| `render.rs` | Inbox → Markdown rendering |
| `file.rs` | File I/O, path resolution |
| `config.rs` | Focus command auto-detection (Zellij/tmux) |
| `tui.rs` | Interactive TUI with crossterm |

## Key Design Decisions

1. **Terminal-agnostic**: No hard dependencies on Zellij/tmux. Focus command via env/flag.
2. **Tool-agnostic**: Generic KV model works with any AI tool that outputs JSON.
3. **No config files**: Everything via CLI flags and env vars. CI-friendly.
4. **Markdown persistence**: Human-readable, easy to debug, simple to parse.
5. **No daemon**: All operations are stateless CLI calls. TUI reads file on demand.
6. **Convention over enforcement**: `pane`, `proj` are conventions, not requirements.

## CLI Reference

```bash
# Add item with attrs
tael add -a "msg=hello" -a "pane=42" -a "proj=myproj"

# Add with JSON stdin (@ prefix extracts from JSON)
echo '{"message":"hello"}' | tael add -a "msg=@.message" -a "pane=42"

# Claude Code preset
echo "$JSON" | tael add --from-claude-code -a "pane=$ZELLIJ_PANE_ID"

# Remove item
tael remove -a pane=42

# List items (flat or grouped)
tael list
tael list --group-by status,proj

# TUI (default command)
tael tui --group-by proj

# Global options
tael --focus-cmd "tmux select-pane -t {pane_id}" tui
```

## Development

```bash
# Build
cargo build

# Test
cargo test

# Run TUI
cargo run

# Run with args
cargo run -- add -a "msg=test" -a "pane=1" -a "proj=myproj"
```

## Code Style

- Standard Rust conventions (`snake_case`, `PascalCase`)
- Use `thiserror` for error types if adding new error handling
- Keep modules focused and small
- Tests go in the same file under `#[cfg(test)]`

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/). Scopes: `tui`, `cli`, `parse`, `config`, `file`, `ci`.

## File Format

Inbox is stored as Markdown with inline attrs:

```markdown
## Waiting

- [ ] Claude needs permission [pane:: 42] [type:: permission_prompt] [proj:: tael]
- [ ] Review helm chart [pane:: 17] [proj:: k3s] [branch:: main]

## Working

- [/] Processing files [pane:: 5] [proj:: tael] [agent:: indexer]
```

- `[ ]` = Waiting (needs user input)
- `[/]` = Working (background task)
- `[key:: value]` = Arbitrary attributes (one per bracket)
- Common attrs: `pane`, `proj`, `branch`, `type`, `agent`

## Testing

```bash
cargo test                    # Run all tests
cargo test parse              # Run parse tests
cargo test --release          # Release mode
```

## Common Tasks

### Adding a new command
1. Add variant to `Commands` enum in `main.rs`
2. Add match arm in `run()` function
3. Update README usage section

### Adding a tool preset (--from-X)
1. Add flag to `AddArgs` in `main.rs`
2. Define attr mappings for the tool's JSON format
3. Document in README integrations section

### Changing TUI rendering
1. Modify `render()` or `render_items()` in `tui.rs`
2. Update snapshot tests if output format changes

### Adding a new attr transform
1. Add transform function in `main.rs` (e.g., `filename`, `lowercase`)
2. Add to `apply_transform()` dispatch

## CI

GitHub Actions runs on every push/PR:
- `cargo test` - All tests
- `cargo clippy` - Lints
- `cargo fmt --check` - Formatting
- Cross-platform builds (Linux, macOS, Windows)
