/// Events produced by the IRC client and surfaced to your application.
/// Match on these in your main loop to drive UI, bot logic, etc.
#[derive(Debug, Clone)]
pub enum Event {
    /// Successfully registered with the server (001 received)
    Connected { server: String, nick: String },

    /// A PRIVMSG or NOTICE in a channel or as a PM
    Message {
        from: String,
        target: String,
        text: String,
        is_notice: bool,
    },

    /// A system message like MOTD
    SysMessage { text: String },

    /// We joined a channel
    Joined { nick: String },

    /// We or someone else left a channel
    Parted { nick: String },

    /// Someone quit the server
    Quit { nick: String },

    /// A nick change (could be ours)
    NickChanged { old_nick: String, new_nick: String },

    /// Channel topic was set or changed
    Topic { channel: String, topic: String },

    /// NAMES list entry (members of a channel)
    Names {
        channel: String,
        members: Vec<String>,
    },

    /// A raw message we didn't handle specifically
    /// Useful for debugging or handling custom commands
    Raw(crate::proto::message::IrcMessage),

    /// The connection was closed
    Disconnected,
}
