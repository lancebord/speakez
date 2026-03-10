use ratatui::widgets::{ListState, ScrollbarState};

/// A single chat message in the log
#[derive(Clone)]
pub struct ChatLine {
    pub nick: String,
    pub text: String,
    pub is_system: bool,
    pub is_join_leave: bool,
    pub is_notice: bool,
}

/// All mutable state for the TUI
pub struct AppState {
    pub nick: String,
    pub channel: String,
    pub messages: Vec<ChatLine>,
    pub members: Vec<String>,
    pub input: String,
    pub cursor: usize,
    pub chat_scroll: usize,
    pub members_scroll: ScrollbarState,
    pub members_list_state: ListState,
    pub status: String,
    pub connected: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            nick: String::new(),
            channel: String::new(),
            messages: Vec::new(),
            members: Vec::new(),
            input: String::new(),
            cursor: 0,
            chat_scroll: 0,
            members_scroll: ScrollbarState::new(0),
            members_list_state: ListState::default(),
            status: "Set nick with /nick to connect.".into(),
            connected: false,
        }
    }

    pub fn push_message(&mut self, nick: &str, text: &str) {
        self.messages.push(ChatLine {
            nick: nick.to_string(),
            text: text.to_string(),
            is_system: false,
            is_join_leave: false,
            is_notice: false,
        });
    }

    pub fn push_system(&mut self, text: &str) {
        self.messages.push(ChatLine {
            nick: String::new(),
            text: text.to_string(),
            is_system: true,
            is_join_leave: false,
            is_notice: false,
        });
    }

    pub fn push_join_leave(&mut self, text: &str) {
        if let Some(m) = self.messages.last_mut() {
            if m.is_join_leave {
                m.text += format!(" {text}").as_str();
                return;
            }
        }

        self.messages.push(ChatLine {
            nick: String::new(),
            text: text.to_string(),
            is_system: true,
            is_join_leave: true,
            is_notice: false,
        });
    }

    pub fn input_insert(&mut self, ch: char) {
        self.input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn input_backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        // Find the start of the previous character
        let prev = self.input[..self.cursor]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.input.remove(prev);
        self.cursor = prev;
    }

    pub fn cursor_left(&mut self) {
        self.cursor = self.input[..self.cursor]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    pub fn cursor_right(&mut self) {
        if self.cursor < self.input.len() {
            let ch = self.input[self.cursor..].chars().next().unwrap();
            self.cursor += ch.len_utf8();
        }
    }

    pub fn scroll_up(&mut self) {
        self.chat_scroll = self.chat_scroll.saturating_add(1);
    }

    pub fn scroll_down(&mut self) {
        self.chat_scroll = self.chat_scroll.saturating_sub(1);
    }

    pub fn members_scroll_up(&mut self) {
        let pos = self.members_scroll.get_position().saturating_sub(5);
        self.members_scroll = self.members_scroll.position(pos);
        *self.members_list_state.offset_mut() = pos;
    }

    pub fn members_scroll_down(&mut self) {
        let max = self.members.len().saturating_sub(1);
        let pos = (self.members_scroll.get_position().saturating_add(5)).min(max);
        self.members_scroll = self.members_scroll.position(pos);
        *self.members_list_state.offset_mut() = pos;
    }

    pub fn take_input(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.input)
    }

    pub fn sort_members(&mut self) {
        self.members.sort_by(|a, b| {
            // Strip sigils for sorting (@, +, %)
            let a = a.trim_start_matches(&['@', '+', '%', '~', '&'][..]);
            let b = b.trim_start_matches(&['@', '+', '%', '~', '&'][..]);
            a.to_lowercase().cmp(&b.to_lowercase())
        });
        self.members.dedup();
    }
}
