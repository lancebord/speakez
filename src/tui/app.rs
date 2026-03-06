/// Compile-time channel to join on startup
pub const CHANNEL: &str = "#speakez";

/// A single chat message in the log
#[derive(Clone)]
pub struct ChatLine {
    pub nick: String,
    pub text: String,
    /// true = server/system message (JOIN, PART, topic, etc.)
    pub is_system: bool,
    /// true = NOTICE
    pub is_notice: bool,
    /// true = this is our own message
    pub is_self: bool,
}

/// All mutable state the TUI needs to render and respond to input
pub struct AppState {
    /// Our nick
    pub nick: String,
    /// The active channel name
    pub channel: String,
    /// Chat log for the active channel
    pub messages: Vec<ChatLine>,
    /// Member list for the active channel
    pub members: Vec<String>,
    /// Current contents of the input box
    pub input: String,
    /// Cursor position within `input` (byte index)
    pub cursor: usize,
    /// Scroll offset from the bottom (0 = pinned to latest)
    pub scroll: usize,
    /// Status line text (connection state, errors, etc.)
    pub status: String,
    /// Whether we've fully registered
    pub connected: bool,
}

impl AppState {
    pub fn new(nick: impl Into<String>, channel: impl Into<String>) -> Self {
        Self {
            nick: nick.into(),
            channel: channel.into(),
            messages: Vec::new(),
            members: Vec::new(),
            input: String::new(),
            cursor: 0,
            scroll: 0,
            status: "Connecting...".into(),
            connected: false,
        }
    }

    /// Push a chat message
    pub fn push_message(&mut self, nick: &str, text: &str, is_self: bool) {
        self.messages.push(ChatLine {
            nick: nick.to_string(),
            text: text.to_string(),
            is_system: false,
            is_notice: false,
            is_self,
        });
    }

    /// Push a system/event line (joins, parts, topic changes)
    pub fn push_system(&mut self, text: &str) {
        self.messages.push(ChatLine {
            nick: String::new(),
            text: text.to_string(),
            is_system: true,
            is_notice: false,
            is_self: false,
        });
    }

    /// Insert a character at the cursor
    pub fn input_insert(&mut self, ch: char) {
        self.input.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    /// Delete the character before the cursor
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

    /// Move cursor left one character
    pub fn cursor_left(&mut self) {
        self.cursor = self.input[..self.cursor]
            .char_indices()
            .last()
            .map(|(i, _)| i)
            .unwrap_or(0);
    }

    /// Move cursor right one character
    pub fn cursor_right(&mut self) {
        if self.cursor < self.input.len() {
            let ch = self.input[self.cursor..].chars().next().unwrap();
            self.cursor += ch.len_utf8();
        }
    }

    /// Take the current input, clear the box, return the text
    pub fn take_input(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.input)
    }

    /// Sort and deduplicate the member list
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
