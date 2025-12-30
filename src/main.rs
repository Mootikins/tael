//! Tael - Terminal-agnostic agent inbox
//!
//! Track AI assistant status across terminal panes.

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use tael::{config::Config, file, Inbox, InboxItem, Status};

#[derive(Parser)]
#[command(name = "tael")]
#[command(about = "Terminal-agnostic agent inbox - track AI assistant status")]
#[command(version)]
struct Cli {
    /// Override inbox file path
    #[arg(long, short = 'f', env = "TAEL_INBOX_FILE", global = true)]
    file: Option<PathBuf>,

    /// Focus command template (use {pane_id} placeholder)
    #[arg(long, env = "TAEL_FOCUS_CMD", global = true)]
    focus_cmd: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add or update an item
    Add {
        /// Attributes in key=value format. Use @.field for JSON stdin extraction.
        #[arg(long = "attr", short = 'a', value_name = "KEY=VALUE")]
        attrs: Vec<String>,

        /// Preset for Claude Code JSON format
        #[arg(long)]
        from_claude_code: bool,

        /// Status: wait or work (default: wait)
        #[arg(long, short = 's', default_value = "wait")]
        status: String,
    },

    /// Remove an item
    Remove {
        /// Attributes to match for removal (e.g., pane=42)
        #[arg(long = "attr", short = 'a', value_name = "KEY=VALUE")]
        attrs: Vec<String>,
    },

    /// List all items
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Group by attribute (e.g., proj, status)
        #[arg(long, value_delimiter = ',')]
        group_by: Vec<String>,
    },

    /// Clear all items
    Clear,

    /// Open interactive TUI
    #[command(alias = "ui")]
    Tui {
        /// Group by attribute (e.g., proj, status)
        #[arg(long, value_delimiter = ',')]
        group_by: Vec<String>,
    },
}

/// Extract value from JSON using @.field syntax
fn extract_json_value(json: &serde_json::Value, expr: &str) -> Option<String> {
    // Simple path extraction: @.field or @.nested.field
    let path = expr.strip_prefix("@.")?;

    // Handle pipe transforms: @.field | transform
    let (path, transform) = if let Some(idx) = path.find(" | ") {
        (&path[..idx], Some(path[idx + 3..].trim()))
    } else {
        (path, None)
    };

    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json;
    for part in parts {
        current = current.get(part)?;
    }

    let value = current
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| current.to_string());

    match transform {
        Some(t) => Some(apply_transform(&value, t)),
        None => Some(value),
    }
}

fn apply_transform(value: &str, transform: &str) -> String {
    match transform {
        "filename" => std::path::Path::new(value)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(value)
            .to_string(),
        "lowercase" => value.to_lowercase(),
        "uppercase" => value.to_uppercase(),
        _ => value.to_string(),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = Config::new(cli.focus_cmd);

    let path = cli.file.unwrap_or_else(file::default_path);

    // Default to TUI if no subcommand
    let command = cli.command.unwrap_or(Commands::Tui { group_by: vec![] });

    match command {
        Commands::Add {
            attrs,
            from_claude_code,
            status,
        } => {
            let status = match status.as_str() {
                "wait" | "waiting" => Status::Waiting,
                "work" | "working" => Status::Working,
                other => {
                    return Err(format!("invalid status '{}': use 'wait' or 'work'", other).into())
                }
            };

            // Read stdin if any attr uses @. syntax or from_claude_code
            let stdin_json: Option<serde_json::Value> =
                if from_claude_code || attrs.iter().any(|a| a.contains("=@.")) {
                    use std::io::Read;
                    let mut input = String::new();
                    std::io::stdin().read_to_string(&mut input)?;
                    Some(serde_json::from_str(&input)?)
                } else {
                    None
                };

            // Parse attrs
            let mut item_attrs = std::collections::HashMap::new();

            // Apply preset if requested
            if from_claude_code {
                if let Some(ref json) = stdin_json {
                    if let Some(v) = extract_json_value(json, "@.message") {
                        item_attrs.insert("msg".to_string(), v);
                    }
                    if let Some(v) = extract_json_value(json, "@.notification_type") {
                        item_attrs.insert("type".to_string(), v);
                    }
                }
            }

            // Parse explicit attrs
            for attr in attrs {
                let (key, value) = attr
                    .split_once('=')
                    .ok_or_else(|| format!("invalid attr '{}': expected key=value", attr))?;

                let resolved_value = if value.starts_with("@.") {
                    stdin_json
                        .as_ref()
                        .and_then(|json| extract_json_value(json, value))
                        .ok_or_else(|| format!("failed to extract '{}' from JSON stdin", value))?
                } else {
                    value.to_string()
                };

                item_attrs.insert(key.to_string(), resolved_value);
            }

            let mut inbox = file::load(&path)?;
            inbox.upsert(InboxItem::new(item_attrs.clone(), status));
            file::save(&path, &inbox)?;

            // Print confirmation
            if let Some(pane) = item_attrs.get("pane") {
                println!("Added item for pane {}", pane);
            } else {
                println!("Added item");
            }
        }

        Commands::Remove { attrs } => {
            // Find pane attr
            let pane = attrs
                .iter()
                .find_map(|a| a.strip_prefix("pane=").and_then(|v| v.parse::<u32>().ok()))
                .ok_or("pane attr required (use -a pane=N)")?;

            let mut inbox = file::load(&path)?;
            if inbox.remove(pane) {
                file::save(&path, &inbox)?;
                println!("Removed item for pane {}", pane);
            } else {
                println!("No item found for pane {}", pane);
            }
        }

        Commands::List { json, group_by } => {
            use std::io::IsTerminal;
            let inbox = file::load(&path)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&inbox)?);
            } else {
                let width = ratatui::crossterm::terminal::size()
                    .map(|(w, _)| w as usize)
                    .unwrap_or(80);
                let is_tty = std::io::stdout().is_terminal();
                print!(
                    "{}",
                    tael::tui::render_list(&inbox, width, is_tty, &group_by)
                );
            }
        }

        Commands::Clear => {
            let inbox = Inbox::new();
            file::save(&path, &inbox)?;
            println!("Cleared inbox");
        }

        Commands::Tui { group_by } => {
            tael::tui::run_interactive(&config, &group_by)?;
        }
    }

    Ok(())
}
