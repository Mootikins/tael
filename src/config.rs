//! Configuration for tael

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::{env, fs};

/// Tael configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Command to focus a pane. Use {pane_id} as placeholder.
    /// Examples:
    /// - Zellij: "zellij action focus-pane-with-id {pane_id}"
    /// - tmux: "tmux select-pane -t {pane_id}"
    pub focus_command: Option<String>,

    /// Checkbox style: "brackets", "circles", "bullets", or "none"
    pub checkbox_style: String,

    /// Enable colors in TUI
    pub colors: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            focus_command: None,
            checkbox_style: "brackets".to_string(),
            colors: true,
        }
    }
}

impl Config {
    /// Load config from file or return defaults
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }

        // Check environment for focus command
        let mut config = Self::default();
        if let Ok(cmd) = env::var("TAEL_FOCUS_CMD") {
            config.focus_command = Some(cmd);
        }

        // Auto-detect multiplexer if no focus command set
        if config.focus_command.is_none() {
            config.focus_command = Self::detect_focus_command();
        }

        config
    }

    /// Get config file path
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tael")
            .join("config.toml")
    }

    /// Auto-detect focus command based on environment
    fn detect_focus_command() -> Option<String> {
        if env::var("ZELLIJ").is_ok() {
            Some("zellij action focus-pane-with-id {pane_id}".to_string())
        } else if env::var("TMUX").is_ok() {
            Some("tmux select-pane -t {pane_id}".to_string())
        } else {
            None
        }
    }

    /// Execute focus command for a pane
    pub fn focus_pane(&self, pane_id: u32) -> Result<(), String> {
        let cmd = self
            .focus_command
            .as_ref()
            .ok_or("No focus command configured")?;

        let cmd = cmd.replace("{pane_id}", &pane_id.to_string());

        // Parse and execute command
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty focus command".to_string());
        }

        let status = std::process::Command::new(parts[0])
            .args(&parts[1..])
            .status()
            .map_err(|e| format!("Failed to execute focus command: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err(format!("Focus command exited with: {}", status))
        }
    }
}
