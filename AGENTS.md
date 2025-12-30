# AI Agent Guide for tael

> Instructions for AI agents (Claude, Codex, etc.) working on the tael codebase

## Project Overview

**tael** (**T**erminal-**A**gnostic **E**vent **L**ister) is a lightweight TUI for tracking AI agent status across terminal panes. Named after [Tael](https://zelda.fandom.com/wiki/Tael), the purple fairy from Zelda: Majora's Mask.

## Architecture

Single-binary Rust CLI with these modules:

| Module | Purpose |
|--------|---------|
| `main.rs` | CLI entry point, clap-based commands |
| `types.rs` | `Inbox`, `InboxItem`, `Status` types |
| `parse.rs` | Markdown → Inbox parsing |
| `render.rs` | Inbox → Markdown rendering |
| `file.rs` | File I/O, path resolution |
| `config.rs` | Configuration loading, focus command |
| `tui.rs` | Interactive TUI with crossterm |

## Key Design Decisions

1. **Terminal-agnostic**: No hard dependencies on Zellij/tmux. Focus command is configurable.
2. **Markdown persistence**: Human-readable, easy to debug, simple to parse.
3. **No daemon**: All operations are stateless CLI calls. TUI reads file on demand.
4. **Pane ID as key**: Items are upserted/removed by pane ID, ensuring one entry per pane.

## Development

```bash
# Build
cargo build

# Test
cargo test

# Run TUI
cargo run

# Run with args
cargo run -- add "test" -p 1 --project myproj
```

## Code Style

- Standard Rust conventions (`snake_case`, `PascalCase`)
- Use `thiserror` for error types if adding new error handling
- Keep modules focused and small
- Tests go in the same file under `#[cfg(test)]`

## File Format

Inbox is stored as Markdown:

```markdown
## Waiting for Input

### project-name (branch)
- [ ] agent: message [pane:: 42]

## Background

### project-name
- [/] agent: working on something [pane:: 5]
```

- `[ ]` = Waiting (needs user input)
- `[/]` = Working (background task)
- `[pane:: N]` = Pane ID for focusing

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

### Adding a new config option
1. Add field to `Config` struct in `config.rs`
2. Add to `Default` impl
3. Document in README config section

### Changing TUI rendering
1. Modify `render()` or `render_items()` in `tui.rs`
2. Update snapshot tests if output format changes

## CI

GitHub Actions runs on every push/PR:
- `cargo test` - All tests
- `cargo clippy` - Lints
- `cargo fmt --check` - Formatting
- Cross-platform builds (Linux, macOS, Windows)
