//! TUI rendering for tael using ratatui

use std::io::{self, stdout};
use std::time::Duration;

// Use crossterm directly (with use-dev-tty feature) instead of ratatui's re-export
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

use crate::config::Config;
use crate::Inbox;

/// Run interactive TUI mode
pub fn run_interactive(config: &Config, group_by: &[String]) -> io::Result<()> {
    let path = crate::file::default_path();
    let inbox = crate::file::load(&path)?;

    // Manual terminal setup using crossterm directly (with use-dev-tty feature)
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(inbox, path.to_path_buf(), group_by.to_vec());

    // Event loop
    let result = loop {
        terminal.draw(|frame| draw(frame, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            let evt = event::read()?;

            if let Event::Key(key) = evt {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match (key.code, key.modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => break Ok(()),
                    (KeyCode::Char('c'), KeyModifiers::CONTROL) => break Ok(()),
                    (KeyCode::Char('j'), _) | (KeyCode::Down, _) => app.next(),
                    (KeyCode::Char('k'), _) | (KeyCode::Up, _) => app.previous(),
                    (KeyCode::Char('d'), _) => app.delete_selected(),
                    (KeyCode::Char('r'), _) => app.reload(),
                    (KeyCode::Enter, _) => {
                        if let Some(pane_id) = app.selected_pane_id() {
                            // Restore terminal before focusing
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
                            disable_raw_mode()?;
                            let _ = config.focus_pane(pane_id);
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
        }
    };

    // Restore terminal
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    result
}

struct App {
    inbox: Inbox,
    /// Index into inbox.items (not the visual list)
    selected_item: Option<usize>,
    list_state: ListState,
    path: std::path::PathBuf,
    /// Grouping keys for display
    group_by: Vec<String>,
}

impl App {
    fn new(inbox: Inbox, path: std::path::PathBuf, group_by: Vec<String>) -> Self {
        let selected_item = if inbox.is_empty() { None } else { Some(0) };
        Self {
            inbox,
            selected_item,
            list_state: ListState::default(),
            path,
            group_by,
        }
    }

    fn next(&mut self) {
        if self.inbox.is_empty() {
            return;
        }
        self.selected_item = Some(match self.selected_item {
            Some(i) => (i + 1).min(self.inbox.items.len() - 1),
            None => 0,
        });
    }

    fn previous(&mut self) {
        if self.inbox.is_empty() {
            return;
        }
        self.selected_item = Some(match self.selected_item {
            Some(i) => i.saturating_sub(1),
            None => 0,
        });
    }

    fn selected_pane_id(&self) -> Option<u32> {
        self.selected_item
            .and_then(|i| self.inbox.items.get(i))
            .and_then(|item| item.pane_id())
    }

    fn delete_selected(&mut self) {
        if let Some(pane_id) = self.selected_pane_id() {
            self.inbox.remove(pane_id);
            let _ = crate::file::save(&self.path, &self.inbox);
            // Adjust selection
            if let Some(i) = self.selected_item {
                if i >= self.inbox.items.len() && !self.inbox.is_empty() {
                    self.selected_item = Some(self.inbox.items.len() - 1);
                } else if self.inbox.is_empty() {
                    self.selected_item = None;
                }
            }
        }
    }

    fn reload(&mut self) {
        if let Ok(inbox) = crate::file::load(&self.path) {
            self.inbox = inbox;
            if let Some(i) = self.selected_item {
                if i >= self.inbox.items.len() {
                    self.selected_item = if self.inbox.is_empty() {
                        None
                    } else {
                        Some(self.inbox.items.len() - 1)
                    };
                }
            }
        }
    }
}

fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Split area: hints (1 line) + separator (1 line) + content
    let chunks = Layout::vertical([
        Constraint::Length(1), // hints
        Constraint::Length(1), // separator line
        Constraint::Min(1),    // content
    ])
    .split(area);

    // Hints line
    let hints = Line::from(vec![
        Span::styled("j/k", Style::default().fg(Color::Yellow)),
        Span::raw(":nav  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(":focus  "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(":del  "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(":reload  "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(":quit"),
    ]);
    frame.render_widget(
        Paragraph::new(hints).style(Style::default().fg(Color::DarkGray)),
        chunks[0],
    );

    // Separator
    let sep = "─".repeat(chunks[1].width as usize);
    frame.render_widget(
        Paragraph::new(sep).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );

    // Content area
    if app.inbox.is_empty() {
        let empty = Paragraph::new("  (no items)").style(
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        );
        frame.render_widget(empty, chunks[2]);
    } else {
        // Build list items with section headers inline, get mapping
        let (items, item_to_visual) = build_list_items(&app.inbox, &app.group_by);

        // Set visual index from selected item
        if let Some(item_idx) = app.selected_item {
            if let Some(&visual_idx) = item_to_visual.get(item_idx) {
                app.list_state.select(Some(visual_idx));
            }
        }

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(list, chunks[2], &mut app.list_state);
    }
}

/// Returns (visual list items, mapping from inbox item index to visual index)
fn build_list_items(inbox: &Inbox, group_by: &[String]) -> (Vec<ListItem<'static>>, Vec<usize>) {
    let mut items = Vec::new();
    let mut item_to_visual = Vec::new(); // item_to_visual[inbox_idx] = visual_idx

    // If no grouping specified, render flat list
    if group_by.is_empty() {
        for item in inbox.items.iter() {
            item_to_visual.push(items.len());
            let item_line = Line::from(format!("[ ] {}", item.msg()));
            items.push(ListItem::new(item_line));
        }
        return (items, item_to_visual);
    }

    // Track current group values for each level
    let mut current_groups: Vec<Option<String>> = vec![None; group_by.len()];

    for item in inbox.items.iter() {
        // Check each grouping level and emit headers as needed
        for (level, key) in group_by.iter().enumerate() {
            let value = get_group_value(item, key);

            if current_groups[level].as_ref() != Some(&value) {
                // Reset all deeper levels when this level changes
                for g in current_groups.iter_mut().take(group_by.len()).skip(level) {
                    *g = None;
                }
                current_groups[level] = Some(value.clone());

                // Emit header with appropriate indentation and color
                let indent = "  ".repeat(level);
                let (color, modifier) = match level {
                    0 => (Color::Yellow, Modifier::BOLD),
                    1 => (Color::Magenta, Modifier::empty()),
                    _ => (Color::Cyan, Modifier::empty()),
                };
                let header_line = Line::from(Span::styled(
                    format!("{}{}", indent, value),
                    Style::default().fg(color).add_modifier(modifier),
                ));
                items.push(ListItem::new(header_line));
            }
        }

        // Item line - indent based on group depth, record its visual index
        item_to_visual.push(items.len());
        let base_indent = "  ".repeat(group_by.len());
        let item_line = Line::from(format!("{}[ ] {}", base_indent, item.msg()));
        items.push(ListItem::new(item_line));
    }

    (items, item_to_visual)
}

/// Get a grouping key value from an item for the given group key
fn get_group_value(item: &crate::InboxItem, key: &str) -> String {
    match key {
        "status" => item.status.section_name().to_string(),
        "proj" => match item.branch() {
            Some(b) => format!("{} ({})", item.proj().unwrap_or("(no project)"), b),
            None => item.proj().unwrap_or("(no project)").to_string(),
        },
        other => item.get(other).unwrap_or("(none)").to_string(),
    }
}

/// Render inbox for non-interactive list output (respects terminal width)
pub fn render_list(inbox: &Inbox, width: usize, colors: bool, group_by: &[String]) -> String {
    let mut output = String::new();

    if inbox.is_empty() {
        if colors {
            output.push_str("\x1b[2m(no items)\x1b[0m\n");
        } else {
            output.push_str("(no items)\n");
        }
        return output;
    }

    // If no grouping specified, render flat list
    if group_by.is_empty() {
        for (idx, item) in inbox.items.iter().enumerate() {
            let prefix = if idx == 0 { "▶ [ ] " } else { "  [ ] " };
            let max_len = width.saturating_sub(prefix.len());
            let text: String = item.msg().chars().take(max_len).collect();
            output.push_str(&format!("{}{}\n", prefix, text));
        }
        return output;
    }

    // Track current group values for each level
    let mut current_groups: Vec<Option<String>> = vec![None; group_by.len()];

    for (idx, item) in inbox.items.iter().enumerate() {
        // Check each grouping level and emit headers as needed
        for (level, key) in group_by.iter().enumerate() {
            let value = get_group_value(item, key);

            if current_groups[level].as_ref() != Some(&value) {
                // Reset all deeper levels when this level changes
                for g in current_groups.iter_mut().take(group_by.len()).skip(level) {
                    *g = None;
                }
                current_groups[level] = Some(value.clone());

                // Emit header with appropriate indentation
                let indent = "  ".repeat(level);
                if colors {
                    // Use yellow for first level, magenta for second, cyan for deeper
                    let color = match level {
                        0 => "1;33", // bold yellow
                        1 => "35",   // magenta
                        _ => "36",   // cyan
                    };
                    output.push_str(&format!("{}\x1b[{}m{}\x1b[0m\n", indent, color, value));
                } else {
                    output.push_str(&format!("{}{}\n", indent, value));
                }
            }
        }

        // Item with truncation - indent based on group depth
        let base_indent = "  ".repeat(group_by.len());
        let prefix = if idx == 0 {
            format!("{}▶ [ ] ", base_indent)
        } else {
            format!("{}  [ ] ", base_indent)
        };
        let max_len = width.saturating_sub(prefix.len());
        let text: String = item.msg().chars().take(max_len).collect();
        output.push_str(&format!("{}{}\n", prefix, text));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_utils::make_item, Status};

    fn sample_inbox() -> Inbox {
        Inbox {
            items: vec![
                make_item(
                    "claude: Auth question",
                    42,
                    "crucible",
                    None,
                    Status::Waiting,
                ),
                make_item(
                    "claude: Review PR",
                    17,
                    "tael",
                    Some("master"),
                    Status::Waiting,
                ),
            ],
        }
    }

    #[test]
    fn render_list_empty() {
        let inbox = Inbox::new();
        let output = render_list(&inbox, 80, false, &[]);
        assert!(output.contains("(no items)"));
    }

    #[test]
    fn render_list_flat() {
        let inbox = sample_inbox();
        let output = render_list(&inbox, 80, false, &[]);
        // Flat list should contain items but not status/project headers
        assert!(output.contains("Auth question"));
        assert!(output.contains("Review PR"));
        assert!(!output.contains("Waiting for Input"));
    }

    #[test]
    fn render_list_with_status_grouping() {
        let inbox = sample_inbox();
        let group_by = vec!["status".to_string()];
        let output = render_list(&inbox, 80, false, &group_by);
        assert!(output.contains("Waiting for Input"));
        assert!(output.contains("Auth question"));
    }

    #[test]
    fn render_list_with_status_proj_grouping() {
        let inbox = sample_inbox();
        let group_by = vec!["status".to_string(), "proj".to_string()];
        let output = render_list(&inbox, 80, false, &group_by);
        assert!(output.contains("Waiting for Input"));
        assert!(output.contains("crucible"));
        assert!(output.contains("tael (master)"));
        assert!(output.contains("Auth question"));
    }

    #[test]
    fn render_list_with_proj_grouping() {
        let inbox = sample_inbox();
        let group_by = vec!["proj".to_string()];
        let output = render_list(&inbox, 80, false, &group_by);
        assert!(output.contains("crucible"));
        assert!(output.contains("tael (master)"));
        assert!(output.contains("Auth question"));
        // Status header should not be present with only proj grouping
        assert!(!output.contains("Waiting for Input"));
    }
}
