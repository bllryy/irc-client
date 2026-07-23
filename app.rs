#[derive(Debug)]
pub enum Event {
    Line {
        buffer: String,
        nick: String,
        text: String,
    },
    Action {
        buffer: String,
        nick: String,
        text: String,
    },
    Status {
        buffer: String,
        text: String,
    },
    Names {
        buffer: String,
        nicks: Vec<String>,
    },
    Join {
        buffer: String,
        nick: String,
    },
    Part {
        buffer: String,
        nick: String,
    },
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Privmsg,
    Action,
    Status,
}

pub struct Msg {
    pub ts: String,
    pub nick: Option<String>,
    pub text: String,
    pub kind: Kind,
}

pub struct Buffer {
    pub name: String,
    pub msgs: Vec<Msg>,
    pub nicks: Vec<String>,
    pub scroll: usize,
    pub unread: bool,
    pub highlight: bool,
}

impl Buffer {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            msgs: Vec::new(),
            nicks: Vec::new(),
            scroll: 0,
            unread: false,
            highlight: false,
        }
    }
}

pub struct App {
    pub nick: String,
    pub buffers: Vec<Buffer>,
    pub current: usize,
    pub input: String,
    pub cursor: usize,
    pub quit: bool,
}

impl App {
    pub fn new(nick: &str) -> Self {
        Self {
            nick: nick.to_string(),
            buffers: vec![Buffer::new("*status*")],
            current: 0,
            input: String::new(),
            cursor: 0,
            quit: false,
        }
    }

    pub fn buf(&self) -> &Buffer {
        &self.buffers[self.current]
    }

    fn buf_mut(&mut self, name: &str) -> &mut Buffer {
        if let Some(i) = self.buffers.iter().position(|b| b.name == name) {
            return &mut self.buffers[i];
        }
        self.buffers.push(Buffer::new(name));
        self.buffers.last_mut().unwrap()
    }

    pub fn switch(&mut self, i: usize) {
        if i < self.buffers.len() {
            self.current = i;
            let b = &mut self.buffers[i];
            b.unread = false;
            b.highlight = false;
        }
    }

    pub fn next_buffer(&mut self, delta: isize) {
        let n = self.buffers.len() as isize;
        let i = (self.current as isize + delta).rem_euclid(n) as usize;
        self.switch(i);
    }

    pub fn scroll(&mut self, delta: isize, max: usize) {
        let b = &mut self.buffers[self.current];
        b.scroll = (b.scroll as isize + delta).clamp(0, max as isize) as usize;
    }

    pub fn push(&mut self, buffer: &str, msg: Msg) {
        let is_current = self.buffers[self.current].name == buffer;
        let me = self.nick.clone();
        let hl = msg.text.to_lowercase().contains(&me.to_lowercase());
        let b = self.buf_mut(buffer);
        b.msgs.push(msg);
        if b.msgs.len() > 5000 {
            b.msgs.drain(0..1000);
        }
        if !is_current {
            b.unread = true;
            b.highlight |= hl;
        }
    }

    pub fn apply(&mut self, ev: Event) {
        let ts = timestamp();
        match ev {
            Event::Line { buffer, nick, text } => self.push(
                &buffer,
                Msg {
                    ts,
                    nick: Some(nick),
                    text,
                    kind: Kind::Privmsg,
                },
            ),
            Event::Action { buffer, nick, text } => self.push(
                &buffer,
                Msg {
                    ts,
                    nick: Some(nick),
                    text,
                    kind: Kind::Action,
                },
            ),
            Event::Status { buffer, text } => self.push(
                &buffer,
                Msg {
                    ts,
                    nick: None,
                    text,
                    kind: Kind::Status,
                },
            ),
            Event::Names { buffer, nicks } => {
                let b = self.buf_mut(&buffer);
                b.nicks = nicks;
                sort_nicks(&mut b.nicks);
            }
            Event::Join { buffer, nick } => {
                let b = self.buf_mut(&buffer);
                if !b.nicks.contains(&nick) {
                    b.nicks.push(nick.clone());
                    sort_nicks(&mut b.nicks);
                }
                self.push(
                    &buffer,
                    Msg {
                        ts,
                        nick: None,
                        text: format!("--> {nick} joined"),
                        kind: Kind::Status,
                    },
                );
            }
            Event::Part { buffer, nick } => {
                let b = self.buf_mut(&buffer);
                b.nicks.retain(|n| *n != nick);
                self.push(
                    &buffer,
                    Msg {
                        ts,
                        nick: None,
                        text: format!("<-- {nick} left"),
                        kind: Kind::Status,
                    },
                );
            }
        }
    }

    pub fn submit(&mut self) -> Option<String> {
        if self.input.is_empty() {
            return None;
        }
        let line = std::mem::take(&mut self.input);
        self.cursor = 0;
        let target = self.buf().name.clone();
        if !line.starts_with('/') {
            let nick = self.nick.clone();
            self.apply(Event::Line {
                buffer: target,
                nick,
                text: line.clone(),
            });
        }
        Some(line)
    }

    pub fn complete_nick(&mut self) {
        let prefix: String = self.input[..self.cursor]
            .rsplit(' ')
            .next()
            .unwrap_or("")
            .to_lowercase();
        if prefix.is_empty() {
            return;
        }
        let hit = self
            .buf()
            .nicks
            .iter()
            .find(|n| n.trim_start_matches(['@', '+']).to_lowercase().starts_with(&prefix))
            .cloned();
        if let Some(n) = hit {
            let n = n.trim_start_matches(['@', '+']).to_string();
            let start = self.cursor - prefix.len();
            let suffix = if start == 0 { ": " } else { " " };
            self.input
                .replace_range(start..self.cursor, &format!("{n}{suffix}"));
            self.cursor = start + n.len() + suffix.len();
        }
    }
}

fn sort_nicks(nicks: &mut [String]) {
    nicks.sort_by_key(|n| {
        let rank = match n.chars().next() {
            Some('@') => 0,
            Some('+') => 1,
            _ => 2,
        };
        (rank, n.trim_start_matches(['@', '+']).to_lowercase())
    });
}

fn timestamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{:02}:{:02}", (secs / 3600) % 24, (secs / 60) % 60)
}
