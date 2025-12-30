//! TUI rendering for tael using ratatui
//!
//! Provides an inline viewport TUI that doesn't take over the terminal.

use std::io;

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    DefaultTerminal, Frame, TerminalOptions, Viewport,
};

use crate::config::Config;
use crate::{Inbox, Status};

/// Height of the inline TUI (title + hints + separator + content + bottom border)
const TUI_HEIGHT: u16 = 12;

/// Run interactive TUI mode with inline viewport
pub fn run_interactive(config: &Config) -> io::Result<()> {
    let path = crate::file::default_path();
    let inbox = crate::file::load(&path)?;

    // Calculate actual height needed
    let item_count = inbox.items.len();
    let height = (item_count as u16 + 5).min(TUI_HEIGHT).max(6); // min 6 for empty state

    // Use ratatui's init_with_options which handles raw mode automatically
    let mut terminal = ratatui::init_with_options(TerminalOptions {
        viewport: Viewport::Inline(height),
    });

    let result = run_app(&mut terminal, inbox, config, &path);

    // Restore terminal state (disables raw mode, etc.)
    ratatui::restore();
    println!();

    result
}

struct App {
    inbox: Inbox,
    list_state: ListState,
    path: std::path::PathBuf,
}

impl App {
    fn new(inbox: Inbox, path: std::path::PathBuf) -> Self {
        let mut list_state = ListState::default();
        if !inbox.is_empty() {
            list_state.select(Some(0));
        }
        Self { inbox, list_state, path }
    }

    fn next(&mut self) {
        if self.inbox.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1).min(self.inbox.items.len() - 1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.inbox.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn selected_pane_id(&self) -> Option<u32> {
        self.list_state
            .selected()
            .and_then(|i| self.inbox.items.get(i))
            .map(|item| item.pane_id)
    }

    fn delete_selected(&mut self) {
        if let Some(pane_id) = self.selected_pane_id() {
            self.inbox.remove(pane_id);
            let _ = crate::file::save(&self.path, &self.inbox);
            // Adjust selection
            if let Some(i) = self.list_state.selected() {
                if i >= self.inbox.items.len() && !self.inbox.is_empty() {
                    self.list_state.select(Some(self.inbox.items.len() - 1));
                } else if self.inbox.is_empty() {
                    self.list_state.select(None);
                }
            }
        }
    }

    fn reload(&mut self) {
        if let Ok(inbox) = crate::file::load(&self.path) {
            self.inbox = inbox;
            if let Some(i) = self.list_state.selected() {
                if i >= self.inbox.items.len() {
                    self.list_state.select(if self.inbox.is_empty() {
                        None
                    } else {
                        Some(self.inbox.items.len() - 1)
                    });
                }
            }
        }
    }
}

fn run_app(
    terminal: &mut DefaultTerminal,
    inbox: Inbox,
    config: &Config,
    path: &std::path::Path,
) -> io::Result<()> {
    let mut app = App::new(inbox, path.to_path_buf());

    loop {
        terminal.draw(|frame| draw(frame, &mut app))?;

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => break,
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,
                (KeyCode::Char('j'), _) | (KeyCode::Down, _) => app.next(),
                (KeyCode::Char('k'), _) | (KeyCode::Up, _) => app.previous(),
                (KeyCode::Char('d'), _) => app.delete_selected(),
                (KeyCode::Char('r'), _) => app.reload(),
                (KeyCode::Enter, _) => {
                    if let Some(pane_id) = app.selected_pane_id() {
                        // Return pane_id to focus after TUI cleanup
                        return focus_pane_after_exit(config, pane_id);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn focus_pane_after_exit(config: &Config, pane_id: u32) -> io::Result<()> {
    let _ = config.focus_pane(pane_id);
    Ok(())
}

fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Main block with title
    let block = Block::default()
        .title(" Tael ")
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split inner area: hints (1 line) + separator (1 line) + content
    let chunks = Layout::vertical([
        Constraint::Length(1), // hints
        Constraint::Length(1), // separator line
        Constraint::Min(1),    // content
    ])
    .split(inner);

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
    frame.render_widget(Paragraph::new(hints).style(Style::default().fg(Color::DarkGray)), chunks[0]);

    // Separator
    let sep = "─".repeat(chunks[1].width as usize);
    frame.render_widget(
        Paragraph::new(sep).style(Style::default().fg(Color::DarkGray)),
        chunks[1],
    );

    // Content area
    if app.inbox.is_empty() {
        let empty = Paragraph::new("  (no items)")
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC));
        frame.render_widget(empty, chunks[2]);
    } else {
        // Build list items with section headers inline
        let items = build_list_items(&app.inbox);
        let list = List::new(items)
            .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            .highlight_symbol("▶ ");
        frame.render_stateful_widget(list, chunks[2], &mut app.list_state);
    }
}

fn build_list_items(inbox: &Inbox) -> Vec<ListItem<'static>> {
    let mut items = Vec::new();
    let mut current_status: Option<Status> = None;
    let mut current_project: Option<String> = None;

    for item in &inbox.items {
        // Section header (status change)
        if current_status != Some(item.status) {
            current_status = Some(item.status);
            current_project = None;
            let section = Line::from(Span::styled(
                item.status.section_name(),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
            items.push(ListItem::new(section));
        }

        // Project header
        let proj_key = match &item.branch {
            Some(b) => format!("{} ({})", item.project, b),
            None => item.project.clone(),
        };
        if current_project.as_ref() != Some(&proj_key) {
            current_project = Some(proj_key.clone());
            let proj_line = Line::from(Span::styled(
                format!("  {}", proj_key),
                Style::default().fg(Color::Magenta),
            ));
            items.push(ListItem::new(proj_line));
        }

        // Item line
        let item_line = Line::from(format!("    [ ] {}", item.text));
        items.push(ListItem::new(item_line));
    }

    items
}

/// Render inbox for non-interactive list output (respects terminal width)
pub fn render_list(inbox: &Inbox, width: usize, colors: bool) -> String {
    let mut output = String::new();

    if inbox.is_empty() {
        if colors {
            output.push_str("\x1b[2m(no items)\x1b[0m\n");
        } else {
            output.push_str("(no items)\n");
        }
        return output;
    }

    let mut current_status: Option<Status> = None;
    let mut current_project: Option<String> = None;

    for (idx, item) in inbox.items.iter().enumerate() {
        // Section header
        if current_status != Some(item.status) {
            current_status = Some(item.status);
            current_project = None;
            if colors {
                output.push_str(&format!("\x1b[1;33m{}\x1b[0m\n", item.status.section_name()));
            } else {
                output.push_str(&format!("{}\n", item.status.section_name()));
            }
        }

        // Project header
        let proj_key = match &item.branch {
            Some(b) => format!("{} ({})", item.project, b),
            None => item.project.clone(),
        };
        if current_project.as_ref() != Some(&proj_key) {
            current_project = Some(proj_key.clone());
            if colors {
                output.push_str(&format!("  \x1b[35m{}\x1b[0m\n", proj_key));
            } else {
                output.push_str(&format!("  {}\n", proj_key));
            }
        }

        // Item with truncation
        let prefix = if idx == 0 { "  ▶ [ ] " } else { "    [ ] " };
        let max_len = width.saturating_sub(prefix.len());
        let text: String = item.text.chars().take(max_len).collect();
        output.push_str(&format!("{}{}\n", prefix, text));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InboxItem;

    fn sample_inbox() -> Inbox {
        Inbox {
            items: vec![
                InboxItem {
                    text: "claude: Auth question".to_string(),
                    pane_id: 42,
                    project: "crucible".to_string(),
                    branch: None,
                    status: Status::Waiting,
                },
                InboxItem {
                    text: "claude: Review PR".to_string(),
                    pane_id: 17,
                    project: "tael".to_string(),
                    branch: Some("master".to_string()),
                    status: Status::Waiting,
                },
            ],
        }
    }

    #[test]
    fn render_list_empty() {
        let inbox = Inbox::new();
        let output = render_list(&inbox, 80, false);
        assert!(output.contains("(no items)"));
    }

    #[test]
    fn render_list_with_items() {
        let inbox = sample_inbox();
        let output = render_list(&inbox, 80, false);
        assert!(output.contains("Waiting for Input"));
        assert!(output.contains("crucible"));
        assert!(output.contains("Auth question"));
    }

    #[test]
    fn render_list_with_branch() {
        let inbox = sample_inbox();
        let output = render_list(&inbox, 80, false);
        assert!(output.contains("tael (master)"));
    }
}
