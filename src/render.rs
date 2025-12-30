//! Markdown rendering for inbox files

use crate::{Inbox, Status};

/// Render inbox to markdown with flat attrs
pub fn render(inbox: &Inbox) -> String {
    if inbox.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    let mut current_status: Option<Status> = None;

    for item in &inbox.items {
        // Section header on status change
        if current_status != Some(item.status) {
            current_status = Some(item.status);
            let section_name = match item.status {
                Status::Waiting => "Waiting",
                Status::Working => "Working",
            };
            output.push_str(&format!("## {}\n\n", section_name));
        }

        // Item line: - [x] msg [key:: value]...
        output.push_str(&format!("- [{}] {}", item.status.to_char(), item.msg()));

        // Render attrs in consistent order: pane, proj, branch, then rest alphabetically
        let priority_keys = ["pane", "proj", "branch"];
        for key in priority_keys {
            if let Some(value) = item.get(key) {
                output.push_str(&format!(" [{}:: {}]", key, value));
            }
        }

        // Remaining attrs alphabetically (excluding msg and priority keys)
        let mut other_keys: Vec<_> = item
            .attrs
            .keys()
            .filter(|k| *k != "msg" && !priority_keys.contains(&k.as_str()))
            .collect();
        other_keys.sort();
        for key in other_keys {
            if let Some(value) = item.get(key) {
                output.push_str(&format!(" [{}:: {}]", key, value));
            }
        }

        output.push('\n');
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InboxItem;
    use std::collections::HashMap;

    /// Helper to create an InboxItem with attrs
    fn make_item(
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

    #[test]
    fn render_empty() {
        let inbox = Inbox::new();
        assert_eq!(render(&inbox), "");
    }

    #[test]
    fn render_flat_attrs() {
        let mut attrs = HashMap::new();
        attrs.insert("msg".to_string(), "hello".to_string());
        attrs.insert("pane".to_string(), "42".to_string());
        attrs.insert("proj".to_string(), "tael".to_string());

        let inbox = Inbox {
            items: vec![InboxItem {
                attrs,
                status: Status::Waiting,
            }],
        };

        let output = render(&inbox);
        assert!(output.contains("## Waiting"));
        assert!(output.contains("- [ ] hello"));
        assert!(output.contains("[pane:: 42]"));
        assert!(output.contains("[proj:: tael]"));
        // Should NOT have ### headers
        assert!(!output.contains("###"));
    }

    #[test]
    fn render_single_item() {
        let inbox = Inbox {
            items: vec![make_item(
                "claude-code: Auth question",
                42,
                "crucible",
                None,
                Status::Waiting,
            )],
        };

        let output = render(&inbox);
        assert!(output.contains("## Waiting"));
        assert!(output.contains("- [ ] claude-code: Auth question"));
        assert!(output.contains("[pane:: 42]"));
        assert!(output.contains("[proj:: crucible]"));
        // Should NOT have ### headers
        assert!(!output.contains("###"));
    }

    #[test]
    fn render_with_branch() {
        let inbox = Inbox {
            items: vec![make_item(
                "Feature work",
                42,
                "crucible",
                Some("feat/inbox"),
                Status::Waiting,
            )],
        };

        let output = render(&inbox);
        assert!(output.contains("[proj:: crucible]"));
        assert!(output.contains("[branch:: feat/inbox]"));
        // Should NOT have ### headers with project (branch) format
        assert!(!output.contains("###"));
        assert!(!output.contains("crucible (feat/inbox)"));
    }

    #[test]
    fn render_roundtrip() {
        let inbox = Inbox {
            items: vec![
                make_item(
                    "claude-code: Auth question",
                    42,
                    "crucible",
                    None,
                    Status::Waiting,
                ),
                make_item("indexer: Processing", 5, "crucible", None, Status::Working),
            ],
        };

        let markdown = render(&inbox);
        let parsed = crate::parse::parse(&markdown);

        assert_eq!(parsed.items.len(), inbox.items.len());
        for (orig, parsed) in inbox.items.iter().zip(parsed.items.iter()) {
            assert_eq!(orig.pane_id(), parsed.pane_id());
            assert_eq!(orig.msg(), parsed.msg());
            assert_eq!(orig.status, parsed.status);
        }
    }

    #[test]
    fn render_attr_ordering() {
        // Test that attrs are rendered in consistent order: pane, proj, branch, then alphabetically
        let mut attrs = HashMap::new();
        attrs.insert("msg".to_string(), "test message".to_string());
        attrs.insert("pane".to_string(), "1".to_string());
        attrs.insert("proj".to_string(), "myproj".to_string());
        attrs.insert("branch".to_string(), "main".to_string());
        attrs.insert("agent".to_string(), "claude".to_string());
        attrs.insert("zebra".to_string(), "last".to_string());

        let inbox = Inbox {
            items: vec![InboxItem {
                attrs,
                status: Status::Waiting,
            }],
        };

        let output = render(&inbox);
        // Verify priority order: pane before proj before branch
        let pane_pos = output.find("[pane::").unwrap();
        let proj_pos = output.find("[proj::").unwrap();
        let branch_pos = output.find("[branch::").unwrap();
        assert!(pane_pos < proj_pos);
        assert!(proj_pos < branch_pos);

        // Verify remaining attrs come after priority keys and are alphabetical
        let agent_pos = output.find("[agent::").unwrap();
        let zebra_pos = output.find("[zebra::").unwrap();
        assert!(branch_pos < agent_pos);
        assert!(agent_pos < zebra_pos);
    }

    #[test]
    fn render_multiple_statuses() {
        let inbox = Inbox {
            items: vec![
                make_item("waiting task", 1, "proj1", None, Status::Waiting),
                make_item("working task", 2, "proj2", None, Status::Working),
            ],
        };

        let output = render(&inbox);
        assert!(output.contains("## Waiting"));
        assert!(output.contains("## Working"));
        // Verify sections appear in order (Waiting before Working based on sort)
        let waiting_pos = output.find("## Waiting").unwrap();
        let working_pos = output.find("## Working").unwrap();
        assert!(waiting_pos < working_pos);
    }
}
