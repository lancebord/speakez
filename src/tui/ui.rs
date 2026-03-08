use super::app::AppState;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};
use unicode_width::UnicodeWidthStr;

// ── Colour palette ────────────────────────────────────────────────────────────
// Dark terminal aesthetic: near-black background, cool grey chrome,
// amber accent for our own nick, cyan for others, muted green for system.

pub fn draw(f: &mut Frame, state: &mut AppState) {
    let area = f.area();

    // Fill background
    f.render_widget(Block::default().style(Style::default()), area);

    // ── Outer layout: title bar + body + status bar ───────────────────────
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // body
            Constraint::Length(1), // status bar
        ])
        .split(area);

    draw_titlebar(f, outer[0], state);
    draw_body(f, outer[1], state);
    draw_statusbar(f, outer[2], state);
}

fn draw_titlebar(f: &mut Frame, area: Rect, state: &AppState) {
    let title = Line::from(vec![
        Span::styled(
            "   ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("speakez", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("  │  ", Style::default()),
        Span::styled(
            &state.channel,
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default()),
        Span::styled(&state.nick, Style::default().fg(Color::Green)),
    ]);

    f.render_widget(Paragraph::new(title).style(Style::default()), area);
}

fn draw_body(f: &mut Frame, area: Rect, state: &mut AppState) {
    // Body:  [chat (fill)] | [members (18)]
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),     // centre: chat log + input
            Constraint::Length(18), // right: member list
        ])
        .split(area);

    draw_center(f, cols[0], state);
    draw_members_panel(f, cols[1], state);
}

fn draw_center(f: &mut Frame, area: Rect, state: &mut AppState) {
    // Centre column: chat log on top, input box on bottom
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // chat log
            Constraint::Length(3), // input box
        ])
        .split(area);

    draw_chat_log(f, rows[0], state);
    draw_input(f, rows[1], state);
}

fn draw_chat_log(f: &mut Frame, area: Rect, state: &mut AppState) {
    let lines: Vec<Line> = state
        .messages
        .iter()
        .map(|msg| render_chat_line(msg))
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Green))
        .style(Style::default());

    let inner_width = area.width.saturating_sub(2) as usize;
    let inner_height = area.height.saturating_sub(2) as usize;
    let total_wrapped = count_wrapped_lines(&lines, inner_width);

    let (padded_lines, base_scroll) = if total_wrapped < inner_height {
        let padding = inner_height - total_wrapped;
        let mut padded = vec![Line::raw(""); padding];
        padded.extend(lines);
        (padded, 0u16)
    } else {
        let scroll = total_wrapped.saturating_sub(inner_height);
        (lines, scroll as u16)
    };
    // Max scrollable lines upward from the natural bottom position
    let max_offset = base_scroll as usize;

    // Clamp the offset and write it back so app.rs stays in sync
    state.scroll_offset = state.scroll_offset.clamp(0, max_offset);
    let final_scroll = (base_scroll as i32 - state.scroll_offset as i32) as u16;

    f.render_widget(
        Paragraph::new(Text::from(padded_lines))
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((final_scroll, 0)),
        area,
    );
}

fn count_wrapped_lines(lines: &[Line], width: usize) -> usize {
    if width == 0 {
        return lines.len();
    }
    lines
        .iter()
        .map(|line| {
            let full_text: String = line
                .spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect();

            if full_text.is_empty() {
                return 1;
            }

            let mut row_count = 1;
            let mut current_width = 0;

            for word in full_text.split_inclusive(' ') {
                let word_width = UnicodeWidthStr::width(word);
                if current_width + word_width > width {
                    row_count += 1;
                    current_width = word_width;
                } else {
                    current_width += word_width;
                }
            }
            row_count
        })
        .sum()
}

fn render_chat_line(msg: &super::app::ChatLine) -> Line<'static> {
    if msg.is_system {
        return Line::from(Span::styled(
            format!("  ∙ {}", msg.text),
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
        ));
    }

    let nick_style = if msg.is_notice {
        Style::default().fg(Color::Green)
    } else {
        Style::default()
            .fg(color_from_str(&msg.nick))
            .add_modifier(Modifier::BOLD)
    };

    let nick = format!("{}", msg.nick);

    Line::from(vec![
        Span::styled(nick, nick_style),
        Span::styled("  ", Style::default()),
        Span::styled(msg.text.clone(), Style::default()),
    ])
}

fn color_from_str(s: &str) -> Color {
    let sum: u32 = s.chars().map(|c| c as u32).sum();
    match sum % 6 {
        0 => Color::Red,
        1 => Color::Green,
        2 => Color::Yellow,
        3 => Color::Blue,
        4 => Color::Magenta,
        5 => Color::Cyan,
        _ => unreachable!(),
    }
}

fn draw_input(f: &mut Frame, area: Rect, state: &AppState) {
    // Show a blinking cursor indicator at the cursor position
    let before = &state.input[..state.cursor];
    let after = &state.input[state.cursor..];

    let cursor_char = if after.is_empty() {
        " "
    } else {
        &after[..after.chars().next().map(|c| c.len_utf8()).unwrap_or(1)]
    };
    let after_cursor = if after.is_empty() {
        ""
    } else {
        &after[cursor_char.len()..]
    };

    let line = Line::from(vec![
        Span::styled(before.to_string(), Style::default()),
        Span::styled(
            cursor_char.to_string(),
            Style::default()
                .bg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(after_cursor.to_string(), Style::default()),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(" send ", Style::default()));

    f.render_widget(Paragraph::new(line).block(block), area);
}

fn draw_members_panel(f: &mut Frame, area: Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .members
        .iter()
        .map(|nick| {
            // Highlight ops (@) differently
            let (sigil, rest) = if nick.starts_with('@') {
                ("@", &nick[1..])
            } else if nick.starts_with('+') {
                ("+", &nick[1..])
            } else {
                ("", nick.as_str())
            };

            let sigil_style = if sigil == "@" {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(sigil.to_string(), sigil_style),
                Span::styled(
                    rest.to_string(),
                    Style::default()
                        .fg(color_from_str(nick.as_str()))
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
        })
        .collect();

    let title = format!(" users ({}) ", state.members.len());
    let block = panel_block(&title);

    f.render_widget(List::new(items).block(block), area);
}

fn draw_statusbar(f: &mut Frame, area: Rect, state: &AppState) {
    let (status_text, status_style) = if state.connected {
        ("● connected", Style::default().fg(Color::LightGreen))
    } else {
        ("○ connecting…", Style::default().fg(Color::Gray))
    };

    let line = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(status_text, status_style),
        Span::styled("  │  ", Style::default()),
        Span::styled(&state.status, Style::default()),
        Span::styled("  │  ", Style::default()),
        Span::styled("Ctrl-C quit", Style::default()),
    ]);

    f.render_widget(Paragraph::new(line).style(Style::default()), area);
}

/// Consistent panel block style
fn panel_block(title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(format!(" {} ", title), Style::default()))
        .style(Style::default())
}
