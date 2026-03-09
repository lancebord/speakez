use tokio::sync::mpsc;

use crate::client::event::Event;
use crate::client::handler::handle;
use crate::client::state::ClientState;
use crate::connection::{self, Sender};
use crate::proto::message::{Command, IrcMessage};

pub mod event;
pub mod handler;
pub mod state;

/// Configuration for the IRC client.
pub struct Config {
    /// Server address, e.g. "irc.libera.chat:6667"
    pub server: String,
    /// Desired nick
    pub nick: String,
    /// IRC username (shown in /whois)
    pub user: String,
    /// Real name (shown in /whois)
    pub realname: String,
    /// Optional server password
    pub password: Option<String>,
}

/// The main IRC client.
///
/// Call `Client::connect` to establish a connection, then drive the event
/// loop with `client.next_event().await` in your application loop.
pub struct Client {
    state: ClientState,
    sender: Sender,
    inbox: mpsc::UnboundedReceiver<IrcMessage>,
    config: Config,
}

impl Client {
    /// Connect to the server and begin the registration handshake.
    pub async fn connect(config: Config) -> Result<Self, std::io::Error> {
        let (sender, inbox) = connection::connect(&config.server).await?;
        let state = ClientState::new(&config.nick);

        let client = Self {
            state,
            sender,
            inbox,
            config,
        };
        client.register();
        Ok(client)
    }

    /// Offer a clone of the sender
    pub fn sender(&self) -> Sender {
        self.sender.clone()
    }

    /// Read-only view of current client state.
    pub fn state(&self) -> &ClientState {
        &self.state
    }

    /// Wait for the next event from the server.
    /// Returns `None` if the connection has closed.
    pub async fn next_event(&mut self) -> Option<Event> {
        loop {
            let msg = self.inbox.recv().await?;
            let events = handle(msg, &mut self.state, &self.sender);

            // Return the first event; re-queue the rest
            // (simple approach: process one at a time via recursive buffering)
            if let Some(first) = events.into_iter().next() {
                return Some(first);
            }
            // If no events were produced (e.g. a PING), loop and wait for next message
        }
    }

    /// Send the registration sequence to the server.
    fn register(&self) {
        // Optional server password
        if let Some(pass) = &self.config.password {
            self.sender
                .send(IrcMessage::new(Command::Pass, vec![pass.clone()]));
        }

        // Begin CAP negotiation first — lets us request IRCv3 caps
        // before NICK/USER so the server doesn't rush past registration
        self.sender.send(IrcMessage::new(
            Command::Cap,
            vec!["LS".into(), "302".into()],
        ));

        self.sender.send(IrcMessage::new(
            Command::Nick,
            vec![self.config.nick.clone()],
        ));

        self.sender.send(IrcMessage::new(
            Command::User,
            vec![
                self.config.user.clone(),
                "0".into(),
                "*".into(),
                self.config.realname.clone(),
            ],
        ));
    }
}
