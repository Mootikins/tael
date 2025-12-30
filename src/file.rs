//! File operations for inbox

use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::{parse, render, Inbox};

/// Get the default inbox file path
pub fn default_path() -> PathBuf {
    // Check override first
    if let Ok(path) = env::var("TAEL_INBOX_FILE") {
        return PathBuf::from(path);
    }

    // Build path in data directory
    let base = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("tael");

    // Use session name if available (for multiplexer isolation)
    let session = env::var("ZELLIJ_SESSION_NAME")
        .or_else(|_| env::var("TMUX_PANE").map(|p| format!("tmux-{}", p)))
        .unwrap_or_else(|_| "default".to_string());

    base.join(format!("{}.md", session))
}

/// Load inbox from file (returns empty inbox if file doesn't exist)
pub fn load(path: &Path) -> Result<Inbox, std::io::Error> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(parse::parse(&content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Inbox::new()),
        Err(e) => Err(e),
    }
}

/// Save inbox to file (creates parent dirs, deletes file if empty)
pub fn save(path: &Path, inbox: &Inbox) -> Result<(), std::io::Error> {
    if inbox.is_empty() {
        // Delete file if it exists
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    } else {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = render::render(inbox);
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_utils::make_item, Status};
    use tempfile::TempDir;

    #[test]
    fn load_nonexistent_returns_empty() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.md");
        let inbox = load(&path).unwrap();
        assert!(inbox.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.md");

        let mut inbox = Inbox::new();
        inbox.upsert(make_item(
            "claude-code: Test",
            42,
            "test-project",
            None,
            Status::Waiting,
        ));

        save(&path, &inbox).unwrap();
        assert!(path.exists());

        let loaded = load(&path).unwrap();
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items[0].pane_id(), Some(42));
    }

    #[test]
    fn save_empty_deletes_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.md");

        // Create file first
        fs::write(&path, "test").unwrap();
        assert!(path.exists());

        // Save empty inbox
        let inbox = Inbox::new();
        save(&path, &inbox).unwrap();
        assert!(!path.exists());
    }
}
