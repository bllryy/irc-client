use crate::app::{App, Kind};
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

const NICK_COLORS: [Color; 8] = [
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::LightRed,
    Color::LightGreen,
];

pub const GUTTER: usize = 18;
const NICKLIST_W: u16 = 18;

fn nick_color(nick: &str) -> Color {
    let h = nick.bytes().fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    NICK_COLORS[(h as usize) % NICK_COLORS.len()]
}

pub fn chat_width(area: Rect) -> usize {
    (area.width.saturating_sub(NICKLIST_W) as usize).saturating_sub(GUTTER + 1)
}

pub fn chat_height(area: Rect) -> usize {
    area.height.saturating_sub(2) as usize
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        if cur.is_empty() {
            cur = word.to_string();
        } else if cur.chars().count() + 1 + word.chars().count() <= width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            out.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
        while cur.chars().count() > width {
            let head: String = cur.chars().take(width).collect();
            cur = cur.chars().skip(width).collect();
            out.push(head);
        }
    }
    if !cur.is_empty() || out.is_empty() {
        out.push(cur);
    }
    out
}

pub fn render_lines(app: &App, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for m in &app.buf().msgs {
        let (label, label_style, body_style) = match m.kind {
            Kind::Privmsg => {
                let n = m.nick.clone().unwrap_or_default();
                let s = Style::default().fg(nick_color(&n));
                (format!("<{n}>"), s, Style::default())
            }
            Kind::Action => {
                let n = m.nick.clone().unwrap_or_default();
                (
                    "*".to_string(),
                    Style::default().fg(nick_color(&n)),
                    Style::default().fg(Color::Magenta),
                )
            }
            Kind::Status => (
                "--".to_string(),
                Style::default().fg(Color::DarkGray),
                Style::default().fg(Color::DarkGray),
            ),
        };

        let body = if m.kind == Kind::Action {
            format!("{} {}", m.nick.clone().unwrap_or_default(), m.text)
        } else {
            m.text.clone()
        };

        let label = truncate(&label, GUTTER - 6);
        for (i, chunk) in wrap(&body, width).into_iter().enumerate() {
            let head = if i == 0 {
                vec![
                    Span::styled(format!("{} ", m.ts), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{:>w$} ", label, w = GUTTER - 6), label_style),
                ]
            } else {
                vec![Span::raw(" ".repeat(GUTTER))]
            };
            let mut spans = head;
            spans.push(Span::styled(chunk, body_style));
            lines.push(Line::from(spans));
        }
    }
    lines
}

fn truncate(s: &str, w: usize) -> String {
    if s.chars().count() <= w {
        s.to_string()
    } else {
        s.chars().take(w).collect()
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(NICKLIST_W)])
        .split(rows[1]);

    let tabs: Vec<Span> = app
        .buffers
        .iter()
        .enumerate()
        .flat_map(|(i, b)| {
            let style = if i == app.current {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else if b.highlight {
                Style::default().fg(Color::Black).bg(Color::Red)
            } else if b.unread {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            [
                Span::styled(format!(" {}:{} ", i + 1, b.name), style),
                Span::raw(""),
            ]
        })
        .collect();
    f.render_widget(Paragraph::new(Line::from(tabs)), rows[0]);

    let width = chat_width(area);
    let height = chat_height(area);
    let all = render_lines(app, width);
    let scroll = app.buf().scroll.min(all.len().saturating_sub(1));
    let end = all.len().saturating_sub(scroll);
    let start = end.saturating_sub(height);
    let view: Vec<Line> = all[start..end].to_vec();

    f.render_widget(
        Paragraph::new(view).block(Block::default().borders(Borders::RIGHT)),
        cols[0],
    );

    let nicks: Vec<Line> = app
        .buf()
        .nicks
        .iter()
        .map(|n| {
            let bare = n.trim_start_matches(['@', '+']);
            Line::from(Span::styled(n.clone(), Style::default().fg(nick_color(bare))))
        })
        .collect();
    f.render_widget(
        Paragraph::new(nicks).block(
            Block::default()
                .title(format!(" {} ", app.buf().nicks.len()))
                .borders(Borders::NONE),
        ),
        cols[1],
    );

    let prompt = format!("[{}] ", app.nick);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(prompt.clone(), Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(app.input.clone()),
        ])),
        rows[2],
    );
    f.set_cursor_position(Position::new(
        rows[2].x + (prompt.chars().count() + app.cursor) as u16,
        rows[2].y,
    ));
}
