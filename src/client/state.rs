use std::collections::{HashMap, HashSet};

/// The full state of a connected IRC client.
#[derive(Debug, Default)]
pub struct ClientState {
    pub nick: String,
    pub channels: HashMap<String, Channel>,
    pub caps: HashSet<String>,
    pub server_name: Option<String>,
    pub reg: RegistrationState,
}

impl ClientState {
    pub fn new(nick: impl Into<String>) -> Self {
        Self {
            nick: nick.into(),
            ..Default::default()
        }
    }

    pub fn channel(&self, name: &str) -> Option<&Channel> {
        self.channels.get(&name.to_lowercase())
    }

    pub fn channel_mut(&mut self, name: &str) -> &mut Channel {
        self.channels
            .entry(name.to_lowercase())
            .or_insert_with(|| Channel::new(name))
    }

    pub fn remove_channel(&mut self, name: &str) {
        self.channels.remove(&name.to_lowercase());
    }
}

/// State of the registration handshake.
#[derive(Debug, Default, PartialEq, Eq)]
pub enum RegistrationState {
    #[default]
    CapNegotiation,
    CapPending,
    WaitingForWelcome,
    Registered,
}

/// A joined channel and its current state.
#[derive(Debug)]
pub struct Channel {
    pub name: String,
    pub members: HashSet<String>,
    pub topic: Option<String>,
}

impl Channel {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            members: HashSet::new(),
            topic: None,
        }
    }
}
