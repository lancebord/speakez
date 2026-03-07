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

const BG: Color = Color::Rgb(16, 16, 16);
const ORANGE: Color = Color::Rgb(251, 84, 43); // border / panel bg
const FG: Color = Color::Rgb(204, 204, 204); // main foreground
const GRAY: Color = Color::Rgb(74, 74, 74); // other nicks
const LIGHT_ORANGE: Color = Color::Rgb(255, 122, 89); // system messages

pub fn draw(f: &mut Frame, state: &AppState) {
    let area = f.area();

    // Fill background
    f.render_widget(Block::default().style(Style::default().bg(BG)), area);

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
            Style::default().fg(ORANGE).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "speakez",
            Style::default().fg(FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(FG)),
        Span::styled(
            &state.channel,
            Style::default().fg(FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  ", Style::default().fg(FG)),
        Span::styled(&state.nick, Style::default().fg(ORANGE)),
    ]);

    f.render_widget(Paragraph::new(title).style(Style::default()), area);
}

fn draw_body(f: &mut Frame, area: Rect, state: &AppState) {
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

fn draw_center(f: &mut Frame, area: Rect, state: &AppState) {
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

fn draw_chat_log(f: &mut Frame, area: Rect, state: &AppState) {
    let lines: Vec<Line> = state
        .messages
        .iter()
        .map(|msg| render_chat_line(msg))
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(ORANGE))
        .style(Style::default().bg(BG));

    let inner_width = area.width.saturating_sub(2) as usize;
    let inner_height = area.height.saturating_sub(2) as usize;

    let total_wrapped = count_wrapped_lines(&lines, inner_width);
    let scroll = total_wrapped.saturating_sub(inner_height);
    f.render_widget(
        Paragraph::new(Text::from(lines))
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((scroll as u16, 0)),
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
                .fg(LIGHT_ORANGE)
                .add_modifier(Modifier::DIM),
        ));
    }

    let nick_style = if msg.is_self {
        Style::default().fg(ORANGE).add_modifier(Modifier::BOLD)
    } else if msg.is_notice {
        Style::default().fg(ORANGE)
    } else {
        Style::default().fg(GRAY)
    };

    let nick = format!("{}", msg.nick);

    Line::from(vec![
        Span::styled(nick, nick_style),
        Span::styled("  ", Style::default()),
        Span::styled(msg.text.clone(), Style::default().fg(FG)),
    ])
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
        Span::styled(before.to_string(), Style::default().fg(FG)),
        Span::styled(
            cursor_char.to_string(),
            Style::default().bg(FG).fg(BG).add_modifier(Modifier::BOLD),
        ),
        Span::styled(after_cursor.to_string(), Style::default().fg(FG)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(" send ", Style::default().fg(FG)));

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
                Style::default().fg(ORANGE)
            } else {
                Style::default().fg(FG)
            };

            ListItem::new(Line::from(vec![
                Span::styled(sigil.to_string(), sigil_style),
                Span::styled(rest.to_string(), Style::default().fg(FG)),
            ]))
        })
        .collect();

    let title = format!(" users ({}) ", state.members.len());
    let block = panel_block(&title);

    f.render_widget(List::new(items).block(block), area);
}

fn draw_statusbar(f: &mut Frame, area: Rect, state: &AppState) {
    let (status_text, status_style) = if state.connected {
        ("● connected", Style::default().fg(LIGHT_ORANGE))
    } else {
        ("○ connecting…", Style::default().fg(GRAY))
    };

    let line = Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(status_text, status_style),
        Span::styled("  │  ", Style::default().fg(FG)),
        Span::styled(&state.status, Style::default().fg(FG)),
        Span::styled("  │  ", Style::default().fg(FG)),
        Span::styled("Ctrl-C quit", Style::default().fg(FG)),
    ]);

    f.render_widget(Paragraph::new(line).style(Style::default()), area);
}

/// Consistent panel block style
fn panel_block(title: &str) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(ORANGE))
        .title(Span::styled(
            format!(" {} ", title),
            Style::default().fg(FG),
        ))
        .style(Style::default().bg(BG))
}
