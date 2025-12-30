//! Markdown parsing for inbox files

use regex::Regex;
use std::sync::LazyLock;

use crate::{Inbox, InboxItem, Status};

// Regex patterns
static SECTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^## (.+)$").expect("valid regex"));

// Matches "### project" or "### project (branch)"
static PROJECT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^### ([^\(]+?)(?:\s*\(([^\)]+)\))?$").expect("valid regex"));

// Match item line: - [x] text [key:: value]...
static ITEM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^- \[(.)\] (.+)$").expect("valid regex"));

// Match individual [key:: value] pairs (key cannot contain : or ])
static ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^:\]]+):: ([^\]]+)\]").expect("valid regex"));

/// Parse an inbox from markdown content
pub fn parse(content: &str) -> Inbox {
    let mut inbox = Inbox::new();
    let mut current_status = Status::Waiting;
    let mut current_project = String::new();
    let mut current_branch: Option<String> = None;

    for line in content.lines() {
        let line = line.trim_end();

        // Check for section header
        if let Some(caps) = SECTION_RE.captures(line) {
            let section_name = caps.get(1).unwrap().as_str();
            current_status = match section_name {
                "Waiting for Input" | "Waiting" => Status::Waiting,
                "Background" | "Working" => Status::Working,
                _ => current_status,
            };
            continue;
        }

        // Check for project header (with optional branch)
        if let Some(caps) = PROJECT_RE.captures(line) {
            current_project = caps.get(1).unwrap().as_str().trim().to_string();
            current_branch = caps.get(2).map(|m| m.as_str().to_string());
            continue;
        }

        // Skip project headers (### ...) - no longer used in flat format
        if line.starts_with("###") {
            continue;
        }

        // Check for item
        if let Some(caps) = ITEM_RE.captures(line) {
            let status_char = caps.get(1).unwrap().as_str().chars().next().unwrap();
            let rest = caps.get(2).unwrap().as_str();

            let status = Status::from_char(status_char).unwrap_or(current_status);

            // Extract all [key:: value] attrs
            let mut attrs = std::collections::HashMap::new();
            for attr_cap in ATTR_RE.captures_iter(rest) {
                let key = attr_cap.get(1).unwrap().as_str().trim().to_string();
                let value = attr_cap.get(2).unwrap().as_str().trim().to_string();
                attrs.insert(key, value);
            }

            // Extract message text (everything before first [key:: pattern)
            let msg = if let Some(m) = ATTR_RE.find(rest) {
                rest[..m.start()].trim().to_string()
            } else {
                rest.trim().to_string()
            };
            if !msg.is_empty() {
                attrs.insert("msg".to_string(), msg);
            }

            // Inject project/branch from ### headers if present and not in attrs
            if !current_project.is_empty() && !attrs.contains_key("proj") {
                attrs.insert("proj".to_string(), current_project.clone());
            }
            if let Some(ref branch) = current_branch {
                if !attrs.contains_key("branch") {
                    attrs.insert("branch".to_string(), branch.clone());
                }
            }

            inbox.items.push(InboxItem { attrs, status });
        }
    }

    inbox
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let inbox = parse("");
        assert!(inbox.is_empty());
    }

    #[test]
    fn parse_single_item() {
        let content = r#"## Waiting for Input

### crucible
- [ ] claude-code: Auth question [pane:: 42]
"#;
        let inbox = parse(content);
        assert_eq!(inbox.items.len(), 1);
        assert_eq!(inbox.items[0].msg(), "claude-code: Auth question");
        assert_eq!(inbox.items[0].pane_id(), Some(42));
        assert_eq!(inbox.items[0].proj(), Some("crucible"));
        assert_eq!(inbox.items[0].status, Status::Waiting);
    }

    #[test]
    fn parse_with_branch() {
        let content = r#"## Waiting for Input

### crucible (feat/inbox)
- [ ] claude-code: Feature work [pane:: 42]
"#;
        let inbox = parse(content);
        assert_eq!(inbox.items.len(), 1);
        assert_eq!(inbox.items[0].proj(), Some("crucible"));
        assert_eq!(inbox.items[0].branch(), Some("feat/inbox"));
    }

    #[test]
    fn parse_multiple_sections() {
        let content = r#"## Waiting for Input

### crucible
- [ ] claude-code: Auth question [pane:: 42]

### k3s
- [ ] claude-code: Helm review [pane:: 17]

## Background

### crucible
- [/] indexer: Processing files [pane:: 5]
"#;
        let inbox = parse(content);
        assert_eq!(inbox.items.len(), 3);

        assert_eq!(inbox.items[0].proj(), Some("crucible"));
        assert_eq!(inbox.items[0].status, Status::Waiting);

        assert_eq!(inbox.items[1].proj(), Some("k3s"));
        assert_eq!(inbox.items[1].status, Status::Waiting);

        assert_eq!(inbox.items[2].proj(), Some("crucible"));
        assert_eq!(inbox.items[2].status, Status::Working);
    }

    #[test]
    fn parse_multiple_attrs() {
        let content = r#"## Waiting

- [ ] hello world [pane:: 42] [proj:: tael] [type:: test]
"#;
        let inbox = parse(content);
        assert_eq!(inbox.items.len(), 1);
        assert_eq!(inbox.items[0].get("pane"), Some("42"));
        assert_eq!(inbox.items[0].get("proj"), Some("tael"));
        assert_eq!(inbox.items[0].get("type"), Some("test"));
        assert_eq!(inbox.items[0].msg(), "hello world");
    }

    #[test]
    fn parse_msg_with_brackets() {
        // Edge case: message containing [ should not be truncated
        let content = r#"## Waiting

- [ ] Fix [bug] in parser [pane:: 42]
"#;
        let inbox = parse(content);
        assert_eq!(inbox.items.len(), 1);
        assert_eq!(inbox.items[0].msg(), "Fix [bug] in parser");
        assert_eq!(inbox.items[0].pane_id(), Some(42));
    }
}
