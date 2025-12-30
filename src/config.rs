//! Configuration for tael

use std::env;

/// Tael configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Command to focus a pane. Use {pane_id} as placeholder.
    /// Examples:
    /// - Zellij: "zellij action launch-plugin file:~/.config/zellij/plugins/tael-focus.wasm --floating --configuration pane_id={pane_id}"
    /// - tmux: "tmux select-pane -t {pane_id}"
    pub focus_command: Option<String>,
}

impl Config {
    /// Create config with optional override, falling back to auto-detection
    pub fn new(override_cmd: Option<String>) -> Self {
        Self {
            focus_command: override_cmd.or_else(Self::detect_focus_command),
        }
    }

    /// Auto-detect focus command based on environment
    fn detect_focus_command() -> Option<String> {
        if env::var("ZELLIJ").is_ok() {
            // Use tael-focus plugin with pane_id in configuration
            // Expand ~ to actual home directory
            if let Some(home) = dirs::home_dir() {
                let plugin_path = home.join(".config/zellij/plugins/tael-focus.wasm");
                Some(format!(
                    "zellij action launch-plugin file:{} --floating --configuration pane_id={{pane_id}}",
                    plugin_path.display()
                ))
            } else {
                None
            }
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

        // Parse command with shell-style quoting (handles spaces in arguments)
        let parts = shell_words::split(&cmd)
            .map_err(|e| format!("Failed to parse focus command: {}", e))?;
        if parts.is_empty() {
            return Err("Empty focus command".to_string());
        }

        let status = std::process::Command::new(&parts[0])
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
