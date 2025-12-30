//! TUI rendering for tael
//!
//! Provides both stateless rendering functions and interactive TUI mode.

use std::io::{self, Write};
use std::str::FromStr;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};

use crate::config::Config;
use crate::{Inbox, Status};

/// Unicode characters for TUI elements
pub mod chars {
    pub const SELECTED: char = '▶';
    pub const ELLIPSIS: char = '…';

    // Checkbox styles
    pub const CHECKBOX_EMPTY: &str = "[ ]";
    pub const CIRCLE_EMPTY: &str = "○";
    pub const BULLET: &str = "•";
}

/// ANSI escape codes for styling
pub mod ansi {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";

    // Colors
    pub const CYAN: &str = "\x1b[36m";
    pub const YELLOW: &str = "\x1b[33m";
    pub const GREEN: &str = "\x1b[32m";
    pub const MAGENTA: &str = "\x1b[35m";
}

/// Style for checkbox/bullet indicators
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CheckboxStyle {
    #[default]
    Brackets,
    Circles,
    Bullets,
    None,
}

impl FromStr for CheckboxStyle {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "circles" | "circle" | "dots" => Self::Circles,
            "bullets" | "bullet" => Self::Bullets,
            "none" | "off" => Self::None,
            _ => Self::Brackets,
        })
    }
}

impl CheckboxStyle {
    pub fn indicator(&self) -> &'static str {
        match self {
            Self::Brackets => chars::CHECKBOX_EMPTY,
            Self::Circles => chars::CIRCLE_EMPTY,
            Self::Bullets => chars::BULLET,
            Self::None => "",
        }
    }
}

/// Render options for the TUI
#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub width: usize,
    pub height: usize,
    pub checkbox_style: CheckboxStyle,
    pub colors: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            width: 50,
            height: 20,
            checkbox_style: CheckboxStyle::default(),
            colors: true,
        }
    }
}

/// Render the inbox TUI to a string buffer
pub fn render(inbox: &Inbox, selected: usize, opts: &RenderOptions) -> String {
    let mut output = String::new();
    let width = opts.width.max(20);
    let height = opts.height.max(3);
    let content_height = height.saturating_sub(2);

    // Title
    if opts.colors {
        output.push_str(ansi::BOLD);
        output.push_str(ansi::CYAN);
    }
    output.push_str("Tael - Agent Inbox");
    if opts.colors {
        output.push_str(ansi::RESET);
    }
    output.push('\n');

    if inbox.is_empty() {
        if opts.colors {
            output.push_str(ansi::DIM);
        }
        output.push_str("  (no items)");
        if opts.colors {
            output.push_str(ansi::RESET);
        }
        output.push('\n');
    } else {
        let overflow = render_items(&mut output, inbox, selected, width, content_height, opts);
        if overflow {
            if opts.colors {
                output.push_str(ansi::DIM);
            }
            output.push_str(&format!("  {} more below", chars::ELLIPSIS));
            if opts.colors {
                output.push_str(ansi::RESET);
            }
            output.push('\n');
        }
    }

    // Footer
    if opts.colors {
        output.push_str(ansi::DIM);
    }
    output.push_str("j/k:nav  Enter:focus  q:quit");
    if opts.colors {
        output.push_str(ansi::RESET);
    }
    output.push('\n');

    output
}

fn render_items(
    output: &mut String,
    inbox: &Inbox,
    selected: usize,
    width: usize,
    max_lines: usize,
    opts: &RenderOptions,
) -> bool {
    let mut lines_used = 0;
    let mut current_status: Option<Status> = None;
    let mut current_project: Option<&str> = None;
    let mut truncated = false;

    for (idx, item) in inbox.items.iter().enumerate() {
        let need_section = current_status != Some(item.status);
        let need_project = current_project != Some(&item.project);
        let lines_needed = 1 + if need_section { 1 } else { 0 } + if need_project { 1 } else { 0 };

        if lines_used + lines_needed > max_lines.saturating_sub(1) && idx < inbox.items.len() - 1 {
            truncated = true;
            break;
        }

        // Section header
        if need_section {
            current_status = Some(item.status);
            current_project = None;
            if opts.colors {
                output.push_str(ansi::BOLD);
                output.push_str(ansi::YELLOW);
            }
            output.push_str(item.status.section_name());
            if opts.colors {
                output.push_str(ansi::RESET);
            }
            output.push('\n');
            lines_used += 1;
        }

        // Project header
        if need_project {
            current_project = Some(&item.project);
            output.push_str("  ");
            if opts.colors {
                output.push_str(ansi::MAGENTA);
            }
            let proj_header = match &item.branch {
                Some(branch) => format!("{} ({})", item.project, branch),
                None => item.project.clone(),
            };
            let max_proj_len = width.saturating_sub(4);
            let proj: String = proj_header.chars().take(max_proj_len).collect();
            output.push_str(&proj);
            if opts.colors {
                output.push_str(ansi::RESET);
            }
            output.push('\n');
            lines_used += 1;
        }

        // Item line
        output.push_str("    ");
        let is_selected = idx == selected;

        if is_selected {
            if opts.colors {
                output.push_str(ansi::GREEN);
            }
            output.push(chars::SELECTED);
            if opts.colors {
                output.push_str(ansi::RESET);
            }
        } else {
            output.push(' ');
        }
        output.push(' ');

        let indicator = opts.checkbox_style.indicator();
        output.push_str(indicator);
        if !indicator.is_empty() {
            output.push(' ');
        }

        let prefix_len = 6 + indicator.len() + 1;
        let max_text_len = width.saturating_sub(prefix_len);
        let text: String = item.text.chars().take(max_text_len).collect();
        output.push_str(&text);
        output.push('\n');
        lines_used += 1;
    }

    truncated
}

/// Run interactive TUI mode
pub fn run_interactive(config: &Config) -> io::Result<()> {
    let path = crate::file::default_path();
    let mut inbox = crate::file::load(&path)?;
    let mut selected: usize = 0;

    // Enter raw mode
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, terminal::EnterAlternateScreen, cursor::Hide)?;

    let result = run_loop(&mut stdout, &mut inbox, &mut selected, config, &path);

    // Cleanup
    execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    result
}

fn run_loop(
    stdout: &mut io::Stdout,
    inbox: &mut Inbox,
    selected: &mut usize,
    config: &Config,
    path: &std::path::Path,
) -> io::Result<()> {
    loop {
        // Get terminal size
        let (width, height) = terminal::size()?;
        let opts = RenderOptions {
            width: width as usize,
            height: height as usize,
            checkbox_style: config.checkbox_style.parse().unwrap_or_default(),
            colors: config.colors,
        };

        // Render
        execute!(stdout, terminal::Clear(ClearType::All), cursor::MoveTo(0, 0))?;
        let output = render(inbox, *selected, &opts);
        write!(stdout, "{}", output)?;
        stdout.flush()?;

        // Handle input
        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => break,
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,

                (KeyCode::Char('j'), _) | (KeyCode::Down, _) => {
                    if !inbox.is_empty() && *selected < inbox.items.len() - 1 {
                        *selected += 1;
                    }
                }

                (KeyCode::Char('k'), _) | (KeyCode::Up, _) => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                }

                (KeyCode::Enter, _) => {
                    if let Some(item) = inbox.items.get(*selected) {
                        let pane_id = item.pane_id;
                        // Exit TUI first
                        execute!(stdout, cursor::Show, terminal::LeaveAlternateScreen)?;
                        terminal::disable_raw_mode()?;

                        // Focus pane
                        if let Err(e) = config.focus_pane(pane_id) {
                            eprintln!("Failed to focus pane: {}", e);
                        }
                        return Ok(());
                    }
                }

                (KeyCode::Char('r'), _) => {
                    // Reload inbox
                    *inbox = crate::file::load(path)?;
                    if *selected >= inbox.items.len() {
                        *selected = inbox.items.len().saturating_sub(1);
                    }
                }

                (KeyCode::Char('d'), _) => {
                    // Delete selected item
                    if let Some(item) = inbox.items.get(*selected) {
                        let pane_id = item.pane_id;
                        inbox.remove(pane_id);
                        crate::file::save(path, inbox)?;
                        if *selected >= inbox.items.len() {
                            *selected = inbox.items.len().saturating_sub(1);
                        }
                    }
                }

                _ => {}
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InboxItem;

    fn sample_inbox() -> Inbox {
        Inbox {
            items: vec![
                InboxItem {
                    text: "claude-code: Auth question".to_string(),
                    pane_id: 42,
                    project: "crucible".to_string(),
                    branch: None,
                    status: Status::Waiting,
                },
                InboxItem {
                    text: "claude-code: Review PR".to_string(),
                    pane_id: 17,
                    project: "k3s".to_string(),
                    branch: None,
                    status: Status::Waiting,
                },
            ],
        }
    }

    #[test]
    fn render_empty_inbox() {
        let inbox = Inbox::new();
        let opts = RenderOptions {
            colors: false,
            ..Default::default()
        };
        let output = render(&inbox, 0, &opts);

        assert!(output.contains("Tael"));
        assert!(output.contains("(no items)"));
    }

    #[test]
    fn render_with_items() {
        let inbox = sample_inbox();
        let opts = RenderOptions {
            colors: false,
            ..Default::default()
        };
        let output = render(&inbox, 0, &opts);

        assert!(output.contains("Waiting for Input"));
        assert!(output.contains("crucible"));
        assert!(output.contains("Auth question"));
    }

    #[test]
    fn render_selection_marker() {
        let inbox = sample_inbox();
        let opts = RenderOptions {
            colors: false,
            ..Default::default()
        };

        let output = render(&inbox, 0, &opts);
        let lines: Vec<&str> = output.lines().collect();
        let auth_line = lines.iter().find(|l| l.contains("Auth question")).unwrap();
        assert!(auth_line.contains('▶'));
    }

    #[test]
    fn checkbox_style_from_str() {
        assert_eq!("circles".parse::<CheckboxStyle>().unwrap(), CheckboxStyle::Circles);
        assert_eq!("bullets".parse::<CheckboxStyle>().unwrap(), CheckboxStyle::Bullets);
        assert_eq!("none".parse::<CheckboxStyle>().unwrap(), CheckboxStyle::None);
        assert_eq!("anything".parse::<CheckboxStyle>().unwrap(), CheckboxStyle::Brackets);
    }
}
