//! Core types for agent inbox

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status of an inbox item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Waiting for user input
    Waiting,
    /// Working in background
    Working,
}

impl Status {
    /// Convert status to single character for markdown
    pub fn to_char(self) -> char {
        match self {
            Self::Waiting => ' ',
            Self::Working => '/',
        }
    }

    /// Parse status from single character
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            ' ' => Some(Self::Waiting),
            '/' => Some(Self::Working),
            _ => None,
        }
    }

    /// Section name for TUI display
    pub fn section_name(self) -> &'static str {
        match self {
            Self::Waiting => "Waiting for Input",
            Self::Working => "Background",
        }
    }
}

/// A single inbox item with generic attributes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxItem {
    /// Generic key-value attributes
    pub attrs: HashMap<String, String>,
    /// Current status
    pub status: Status,
}

impl InboxItem {
    /// Create new item with attrs
    pub fn new(attrs: HashMap<String, String>, status: Status) -> Self {
        Self { attrs, status }
    }

    /// Get attribute value
    pub fn get(&self, key: &str) -> Option<&str> {
        self.attrs.get(key).map(|s| s.as_str())
    }

    /// Get pane ID (convention: "pane" attr parsed as u32)
    pub fn pane_id(&self) -> Option<u32> {
        self.get("pane").and_then(|s| s.parse().ok())
    }

    /// Get message text (convention: "msg" attr)
    pub fn msg(&self) -> &str {
        self.get("msg").unwrap_or("")
    }

    /// Get project (convention: "proj" attr)
    pub fn proj(&self) -> Option<&str> {
        self.get("proj")
    }

    /// Get branch (convention: "branch" attr)
    pub fn branch(&self) -> Option<&str> {
        self.get("branch")
    }
}

/// The inbox containing all items
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Inbox {
    pub items: Vec<InboxItem>,
}

impl Inbox {
    /// Create empty inbox
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if inbox is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Add or update an item by pane attr
    pub fn upsert(&mut self, item: InboxItem) {
        if let Some(pane) = item.pane_id() {
            if let Some(existing) = self.items.iter_mut().find(|i| i.pane_id() == Some(pane)) {
                *existing = item;
                self.sort();
                return;
            }
        }
        self.items.push(item);
        self.sort();
    }

    /// Remove an item by pane ID
    pub fn remove(&mut self, pane_id: u32) -> bool {
        let len_before = self.items.len();
        self.items.retain(|i| i.pane_id() != Some(pane_id));
        self.items.len() < len_before
    }

    /// Sort items: Waiting before Working, then by proj
    fn sort(&mut self) {
        self.items.sort_by(|a, b| match (a.status, b.status) {
            (Status::Waiting, Status::Working) => std::cmp::Ordering::Less,
            (Status::Working, Status::Waiting) => std::cmp::Ordering::Greater,
            _ => a.proj().cmp(&b.proj()),
        });
    }
}

/// Test utilities (available to other modules via pub use)
#[cfg(test)]
pub mod test_utils {
    use super::*;

    /// Helper to create an InboxItem with common attrs
    pub fn make_item(
        msg: &str,
        pane: u32,
        proj: &str,
        branch: Option<&str>,
        status: Status,
    ) -> InboxItem {
        let mut attrs = HashMap::new();
        attrs.insert("msg".to_string(), msg.to_string());
        attrs.insert("pane".to_string(), pane.to_string());
        attrs.insert("proj".to_string(), proj.to_string());
        if let Some(b) = branch {
            attrs.insert("branch".to_string(), b.to_string());
        }
        InboxItem { attrs, status }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbox_item_attrs() {
        let mut attrs = std::collections::HashMap::new();
        attrs.insert("msg".to_string(), "hello".to_string());
        attrs.insert("pane".to_string(), "42".to_string());
        attrs.insert("proj".to_string(), "tael".to_string());

        let item = InboxItem {
            attrs,
            status: Status::Waiting,
        };

        assert_eq!(item.get("msg"), Some("hello"));
        assert_eq!(item.get("pane"), Some("42"));
        assert_eq!(item.pane_id(), Some(42));
    }
}
