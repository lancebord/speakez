use std::collections::HashSet;

/// The full state of a connected IRC client.
#[derive(Debug, Default)]
pub struct ClientState {
    pub nick: String,
    pub channel: Channel,
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
#[derive(Debug, Default)]
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
