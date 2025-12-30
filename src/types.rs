//! Core types for agent inbox

use serde::{Deserialize, Serialize};

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

/// A single inbox item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxItem {
    /// Display text (e.g., "claude-code: Auth question")
    pub text: String,
    /// Pane ID (unique key)
    pub pane_id: u32,
    /// Project name
    pub project: String,
    /// Git branch (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Current status
    pub status: Status,
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

    /// Add or update an item by pane_id
    pub fn upsert(&mut self, item: InboxItem) {
        if let Some(existing) = self.items.iter_mut().find(|i| i.pane_id == item.pane_id) {
            *existing = item;
        } else {
            self.items.push(item);
        }
        self.sort();
    }

    /// Remove an item by pane_id
    pub fn remove(&mut self, pane_id: u32) -> bool {
        let len_before = self.items.len();
        self.items.retain(|i| i.pane_id != pane_id);
        self.items.len() < len_before
    }

    /// Sort items: Waiting before Working, then by project
    fn sort(&mut self) {
        self.items.sort_by(|a, b| {
            match (a.status, b.status) {
                (Status::Waiting, Status::Working) => std::cmp::Ordering::Less,
                (Status::Working, Status::Waiting) => std::cmp::Ordering::Greater,
                _ => (&a.project, &a.branch).cmp(&(&b.project, &b.branch)),
            }
        });
    }
}
