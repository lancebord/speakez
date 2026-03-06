use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct IrcMessage {
    pub tags: HashMap<String, Option<String>>,
    pub prefix: Option<Prefix>,
    pub command: Command,
    pub params: Vec<String>,
}

impl IrcMessage {
    pub fn trailing(&self) -> Option<&str> {
        self.params.last().map(|s| s.as_str())
    }

    pub fn new(command: Command, params: Vec<String>) -> Self {
        Self {
            tags: HashMap::new(),
            prefix: None,
            command,
            params,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prefix {
    Server(String),
    User {
        nick: String,
        user: Option<String>,
        host: Option<String>,
    },
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Prefix::Server(s) => write!(f, "{}", s),
            Prefix::User { nick, user, host } => {
                write!(f, "{}", nick)?;
                if let Some(u) = user {
                    write!(f, "!{}", u)?;
                }
                if let Some(h) = host {
                    write!(f, "@{}", h)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Command {
    // connection
    Cap,
    Nick,
    User,
    Pass,
    Quit,
    Ping,
    Pong,

    // channel operations
    Join,
    Part,
    Kick,
    Topic,
    Names,
    List,
    Invite,

    // messaging
    Privmsg,
    Notice,

    // mode & status
    Mode,
    Who,
    Whois,
    Whowas,

    // Server
    Oper,
    Kill,
    Rehash,

    // Numeric (001-999)
    Numeric(u16),

    Other(String),
}

impl Command {
    pub fn from_str(s: &str) -> Self {
        if s.len() == 3 && s.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(n) = s.parse::<u16>() {
                return Command::Numeric(n);
            }
        }

        match s.to_ascii_uppercase().as_str() {
            "CAP" => Command::Cap,
            "NICK" => Command::Nick,
            "USER" => Command::User,
            "PASS" => Command::Pass,
            "QUIT" => Command::Quit,
            "PING" => Command::Ping,
            "PONG" => Command::Pong,
            "JOIN" => Command::Join,
            "PART" => Command::Part,
            "KICK" => Command::Kick,
            "TOPIC" => Command::Topic,
            "NAMES" => Command::Names,
            "LIST" => Command::List,
            "INVITE" => Command::Invite,
            "PRIVMSG" => Command::Privmsg,
            "NOTICE" => Command::Notice,
            "MODE" => Command::Mode,
            "WHO" => Command::Who,
            "WHOIS" => Command::Whois,
            "WHOWAS" => Command::Whowas,
            "OPER" => Command::Oper,
            "KILL" => Command::Kill,
            "REHASH" => Command::Rehash,
            other => Command::Other(other.to_string()),
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Cap => write!(f, "CAP"),
            Command::Nick => write!(f, "NICK"),
            Command::User => write!(f, "USER"),
            Command::Pass => write!(f, "PASS"),
            Command::Quit => write!(f, "QUIT"),
            Command::Ping => write!(f, "PING"),
            Command::Pong => write!(f, "PONG"),
            Command::Join => write!(f, "JOIN"),
            Command::Part => write!(f, "PART"),
            Command::Kick => write!(f, "KICK"),
            Command::Topic => write!(f, "TOPIC"),
            Command::Names => write!(f, "NAMES"),
            Command::List => write!(f, "LIST"),
            Command::Invite => write!(f, "INVITE"),
            Command::Privmsg => write!(f, "PRIVMSG"),
            Command::Notice => write!(f, "NOTICE"),
            Command::Mode => write!(f, "MODE"),
            Command::Who => write!(f, "WHO"),
            Command::Whois => write!(f, "WHOIS"),
            Command::Whowas => write!(f, "WHOWAS"),
            Command::Oper => write!(f, "OPER"),
            Command::Kill => write!(f, "KILL"),
            Command::Rehash => write!(f, "REHASH"),
            Command::Numeric(n) => write!(f, "{:03}", n),
            Command::Other(s) => write!(f, "{}", s),
        }
    }
}
