mod app;
mod ui;

use app::{App, Event};
use futures::StreamExt;
use ratatui::crossterm::event::{
    Event as TermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new("me");
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<String>();

    tokio::spawn(fake_server(tx.clone()));
    tokio::spawn(async move { while cmd_rx.recv().await.is_some() {} });

    let mut term_events = EventStream::new();

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;
        if app.quit {
            break;
        }

        tokio::select! {
            Some(Ok(ev)) = term_events.next() => {
                let area = terminal.get_frame().area();
                let total = ui::render_lines(&app, ui::chat_width(area)).len();
                let max = total.saturating_sub(ui::chat_height(area));
                if let TermEvent::Key(k) = ev {
                    if k.kind == KeyEventKind::Press {
                        on_key(&mut app, k, max, &cmd_tx);
                    }
                }
            }
            Some(ev) = rx.recv() => app.apply(ev),
        }
    }

    ratatui::restore();
    Ok(())
}

fn on_key(app: &mut App, k: KeyEvent, max_scroll: usize, cmd_tx: &mpsc::UnboundedSender<String>) {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let alt = k.modifiers.contains(KeyModifiers::ALT);

    match k.code {
        KeyCode::Char('c') if ctrl => app.quit = true,
        KeyCode::Char(c) if alt && c.is_ascii_digit() => {
            let i = c.to_digit(10).unwrap() as usize;
            if i > 0 {
                app.switch(i - 1);
            }
        }
        KeyCode::Char('n') if ctrl => app.next_buffer(1),
        KeyCode::Char('p') if ctrl => app.next_buffer(-1),
        KeyCode::Char('u') if ctrl => {
            app.input.drain(..app.cursor);
            app.cursor = 0;
        }
        KeyCode::PageUp => app.scroll(10, max_scroll),
        KeyCode::PageDown => app.scroll(-10, max_scroll),
        KeyCode::Tab => app.complete_nick(),
        KeyCode::Left => app.cursor = app.cursor.saturating_sub(1),
        KeyCode::Right => app.cursor = (app.cursor + 1).min(app.input.len()),
        KeyCode::Home => app.cursor = 0,
        KeyCode::End => app.cursor = app.input.len(),
        KeyCode::Backspace => {
            if app.cursor > 0 {
                app.cursor -= 1;
                app.input.remove(app.cursor);
            }
        }
        KeyCode::Delete => {
            if app.cursor < app.input.len() {
                app.input.remove(app.cursor);
            }
        }
        KeyCode::Enter => {
            if let Some(line) = app.submit() {
                let _ = cmd_tx.send(line);
            }
            app.buffers[app.current].scroll = 0;
        }
        KeyCode::Char(c) => {
            app.input.insert(app.cursor, c);
            app.cursor += c.len_utf8();
        }
        _ => {}
    }
}

async fn fake_server(tx: mpsc::UnboundedSender<Event>) {
    let _ = tx.send(Event::Status {
        buffer: "*status*".into(),
        text: "connected to irc.example.net".into(),
    });
    let _ = tx.send(Event::Names {
        buffer: "#rust".into(),
        nicks: vec!["@alice".into(), "+bob".into(), "carol".into(), "dave".into()],
    });
    let chatter = [
        ("alice", "the borrow checker is just type errors with extra steps"),
        ("bob", "me: has anyone actually read RFC 2812 all the way through"),
        ("carol", "only the parts that were still true"),
        ("dave", "PING timeouts are the real protocol"),
    ];
    let mut i = 0usize;
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let (nick, text) = chatter[i % chatter.len()];
        let _ = tx.send(Event::Line {
            buffer: "#rust".into(),
            nick: nick.into(),
            text: text.into(),
        });
        i += 1;
    }
}
