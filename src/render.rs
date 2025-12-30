//! Markdown rendering for inbox files

use std::collections::BTreeMap;

use crate::{Inbox, Status};

/// Key for grouping items by project + branch
fn project_key(item: &crate::InboxItem) -> String {
    match &item.branch {
        Some(branch) => format!("{} ({})", item.project, branch),
        None => item.project.clone(),
    }
}

/// Render inbox to markdown
pub fn render(inbox: &Inbox) -> String {
    if inbox.is_empty() {
        return String::new();
    }

    // Group by status, then by project+branch
    let mut waiting: BTreeMap<String, Vec<&crate::InboxItem>> = BTreeMap::new();
    let mut working: BTreeMap<String, Vec<&crate::InboxItem>> = BTreeMap::new();

    for item in &inbox.items {
        let map = match item.status {
            Status::Waiting => &mut waiting,
            Status::Working => &mut working,
        };
        map.entry(project_key(item)).or_default().push(item);
    }

    let mut output = String::new();

    // Render waiting section
    if !waiting.is_empty() {
        output.push_str("## Waiting for Input\n\n");
        for (project_header, items) in waiting {
            output.push_str(&format!("### {}\n", project_header));
            for item in items {
                output.push_str(&format!(
                    "- [{}] {} [pane:: {}]\n",
                    item.status.to_char(),
                    item.text,
                    item.pane_id
                ));
            }
            output.push('\n');
        }
    }

    // Render working section
    if !working.is_empty() {
        output.push_str("## Background\n\n");
        for (project_header, items) in working {
            output.push_str(&format!("### {}\n", project_header));
            for item in items {
                output.push_str(&format!(
                    "- [{}] {} [pane:: {}]\n",
                    item.status.to_char(),
                    item.text,
                    item.pane_id
                ));
            }
            output.push('\n');
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InboxItem;

    #[test]
    fn render_empty() {
        let inbox = Inbox::new();
        assert_eq!(render(&inbox), "");
    }

    #[test]
    fn render_single_item() {
        let inbox = Inbox {
            items: vec![InboxItem {
                text: "claude-code: Auth question".to_string(),
                pane_id: 42,
                project: "crucible".to_string(),
                branch: None,
                status: Status::Waiting,
            }],
        };

        let output = render(&inbox);
        assert!(output.contains("## Waiting for Input"));
        assert!(output.contains("### crucible"));
        assert!(output.contains("- [ ] claude-code: Auth question [pane:: 42]"));
    }

    #[test]
    fn render_with_branch() {
        let inbox = Inbox {
            items: vec![InboxItem {
                text: "Feature work".to_string(),
                pane_id: 42,
                project: "crucible".to_string(),
                branch: Some("feat/inbox".to_string()),
                status: Status::Waiting,
            }],
        };

        let output = render(&inbox);
        assert!(output.contains("### crucible (feat/inbox)"));
    }

    #[test]
    fn render_roundtrip() {
        let inbox = Inbox {
            items: vec![
                InboxItem {
                    text: "claude-code: Auth question".to_string(),
                    pane_id: 42,
                    project: "crucible".to_string(),
                    branch: None,
                    status: Status::Waiting,
                },
                InboxItem {
                    text: "indexer: Processing".to_string(),
                    pane_id: 5,
                    project: "crucible".to_string(),
                    branch: None,
                    status: Status::Working,
                },
            ],
        };

        let markdown = render(&inbox);
        let parsed = crate::parse::parse(&markdown);

        assert_eq!(parsed.items.len(), inbox.items.len());
        for (orig, parsed) in inbox.items.iter().zip(parsed.items.iter()) {
            assert_eq!(orig.pane_id, parsed.pane_id);
            assert_eq!(orig.text, parsed.text);
            assert_eq!(orig.status, parsed.status);
        }
    }
}
