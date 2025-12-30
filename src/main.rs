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

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Add or update an item
    Add {
        /// Display text (e.g., "claude-code: Waiting for input")
        text: String,

        /// Pane ID (unique key)
        #[arg(long, short = 'p', env = "ZELLIJ_PANE_ID")]
        pane: Option<u32>,

        /// Project name
        #[arg(long)]
        project: String,

        /// Git branch (optional)
        #[arg(long, short = 'b')]
        branch: Option<String>,

        /// Status: wait or work
        #[arg(long, short = 's', default_value = "wait")]
        status: String,
    },

    /// Remove an item
    Remove {
        /// Pane ID to remove
        #[arg(long, short = 'p', env = "ZELLIJ_PANE_ID")]
        pane: Option<u32>,
    },

    /// List all items
    List {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Clear all items
    Clear,

    /// Open interactive TUI
    #[command(alias = "ui")]
    Tui,

    /// Show current config
    Config,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = Config::load();

    let path = cli.file.unwrap_or_else(file::default_path);

    // Default to TUI if no subcommand
    let command = cli.command.unwrap_or(Commands::Tui);

    match command {
        Commands::Add {
            text,
            pane,
            project,
            branch,
            status,
        } => {
            let pane = pane.ok_or("Pane ID required (use --pane or set ZELLIJ_PANE_ID)")?;
            let status = match status.as_str() {
                "wait" | "waiting" => Status::Waiting,
                "work" | "working" => Status::Working,
                other => return Err(format!("invalid status '{}': use 'wait' or 'work'", other).into()),
            };

            let mut inbox = file::load(&path)?;
            inbox.upsert(InboxItem {
                text,
                pane_id: pane,
                project,
                branch,
                status,
            });
            file::save(&path, &inbox)?;
            println!("Added item for pane {}", pane);
        }

        Commands::Remove { pane } => {
            let pane = pane.ok_or("Pane ID required (use --pane or set ZELLIJ_PANE_ID)")?;
            let mut inbox = file::load(&path)?;
            if inbox.remove(pane) {
                file::save(&path, &inbox)?;
                println!("Removed item for pane {}", pane);
            } else {
                println!("No item found for pane {}", pane);
            }
        }

        Commands::List { json } => {
            use std::io::IsTerminal;
            let inbox = file::load(&path)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&inbox)?);
            } else {
                // Use TUI render with terminal width
                let width = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80);
                let is_tty = std::io::stdout().is_terminal();
                let opts = tael::tui::RenderOptions {
                    width,
                    height: 1000, // no height limit for list
                    checkbox_style: config.checkbox_style.parse().unwrap_or_default(),
                    colors: config.colors && is_tty,
                };
                print!("{}", tael::tui::render(&inbox, 0, &opts));
            }
        }

        Commands::Clear => {
            let inbox = Inbox::new();
            file::save(&path, &inbox)?;
            println!("Cleared inbox");
        }

        Commands::Tui => {
            tael::tui::run_interactive(&config)?;
        }

        Commands::Config => {
            println!("Config file: {}", Config::config_path().display());
            println!("Inbox file: {}", path.display());
            println!();
            println!("focus_command: {:?}", config.focus_command);
            println!("checkbox_style: {}", config.checkbox_style);
            println!("colors: {}", config.colors);
        }
    }

    Ok(())
}
