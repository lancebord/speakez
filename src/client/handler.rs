use tracing::{debug, warn};

use crate::client::event::Event;
use crate::client::state::{ClientState, RegistrationState};
use crate::connection::Sender;
use crate::proto::message::{Command, IrcMessage, Prefix};
use crate::proto::serializer::serialize;

/// Dispatch a single incoming `IrcMessage`, updating `state` and returning
/// zero or more `Event`s for the application to handle.
pub fn handle(msg: IrcMessage, state: &mut ClientState, sender: &Sender) -> Vec<Event> {
    let mut events = Vec::new();

    match &msg.command {
        // --- PING: must reply immediately or the server drops us ---
        Command::Ping => {
            let token = msg.params.first().cloned().unwrap_or_default();
            sender.send(IrcMessage::new(Command::Pong, vec![token]));
        }

        // --- CAP: capability negotiation ---
        Command::Cap => {
            handle_cap(&msg, state, sender);
        }

        // --- 001: welcome — registration complete ---
        Command::Numeric(1) => {
            let server = msg
                .prefix
                .as_ref()
                .map(|p| match p {
                    Prefix::Server(s) => s.clone(),
                    Prefix::User { nick, .. } => nick.clone(),
                })
                .unwrap_or_default();

            // Server may have assigned us a different nick
            if let Some(nick) = msg.params.first() {
                state.nick = nick.clone();
            }

            state.reg = RegistrationState::Registered;
            state.server_name = Some(server.clone());

            events.push(Event::Connected {
                server,
                nick: state.nick.clone(),
            });
        }

        // --- 353: NAMES reply (list of members in a channel) ---
        Command::Numeric(353) => {
            // params: [our_nick, ("=" / "*" / "@"), channel, ":member1 member2 ..."]
            if let (Some(channel), Some(members_str)) = (msg.params.get(2), msg.params.get(3)) {
                let members: Vec<String> = members_str
                    .split_whitespace()
                    // Strip membership prefixes (@, +, etc.)
                    .map(|m| {
                        m.trim_start_matches(&['@', '+', '%', '~', '&'][..])
                            .to_string()
                    })
                    .collect();

                let ch = state.channel_mut(channel);
                ch.members.extend(members.clone());

                events.push(Event::Names {
                    channel: channel.clone(),
                    members,
                });
            }
        }

        // --- 332: topic on join ---
        Command::Numeric(332) => {
            if let (Some(channel), Some(topic)) = (msg.params.get(1), msg.params.get(2)) {
                state.channel_mut(channel).topic = Some(topic.clone());
                events.push(Event::Topic {
                    channel: channel.clone(),
                    topic: topic.clone(),
                });
            }
        }

        // --- JOIN ---
        Command::Join => {
            let nick = nick_from_prefix(&msg.prefix);
            if let Some(channel) = msg.params.first() {
                if nick == state.nick {
                    // We joined
                    state.channel_mut(channel);
                    events.push(Event::Joined {
                        channel: channel.clone(),
                    });
                } else {
                    // Someone else joined
                    state.channel_mut(channel).members.insert(nick);
                }
            }
        }

        // --- PART ---
        Command::Part => {
            let nick = nick_from_prefix(&msg.prefix);
            let channel = msg.params.first().cloned().unwrap_or_default();
            let reason = msg.params.get(1).cloned();

            if nick == state.nick {
                state.remove_channel(&channel);
            } else {
                state.channel_mut(&channel).members.remove(&nick);
            }

            events.push(Event::Parted {
                channel,
                nick,
                reason,
            });
        }

        // --- QUIT ---
        Command::Quit => {
            let nick = nick_from_prefix(&msg.prefix);
            let reason = msg.params.first().cloned();

            // Remove them from all channels
            for ch in state.channels.values_mut() {
                ch.members.remove(&nick);
            }

            events.push(Event::Quit { nick, reason });
        }

        // --- NICK ---
        Command::Nick => {
            let old_nick = nick_from_prefix(&msg.prefix);
            let new_nick = msg.params.first().cloned().unwrap_or_default();

            if old_nick == state.nick {
                state.nick = new_nick.clone();
            }

            // Update in all channels
            for ch in state.channels.values_mut() {
                if ch.members.remove(&old_nick) {
                    ch.members.insert(new_nick.clone());
                }
            }

            events.push(Event::NickChanged { old_nick, new_nick });
        }

        // --- PRIVMSG / NOTICE ---
        Command::Privmsg | Command::Notice => {
            let from = nick_from_prefix(&msg.prefix);
            let target = msg.params.first().cloned().unwrap_or_default();
            let text = msg.params.get(1).cloned().unwrap_or_default();
            let is_notice = msg.command == Command::Notice;

            events.push(Event::Message {
                from,
                target,
                text,
                is_notice,
            });
        }

        // --- TOPIC (live change) ---
        Command::Topic => {
            if let (Some(channel), Some(topic)) = (msg.params.first(), msg.params.get(1)) {
                state.channel_mut(channel).topic = Some(topic.clone());
                events.push(Event::Topic {
                    channel: channel.clone(),
                    topic: topic.clone(),
                });
            }
        }

        // --- Everything else: surface as Raw ---
        _ => {
            debug!("Unhandled: {}", serialize(&msg));
            events.push(Event::Raw(msg));
        }
    }

    events
}

/// Handle CAP sub-commands during capability negotiation.
fn handle_cap(msg: &IrcMessage, state: &mut ClientState, sender: &Sender) {
    // params: [target, subcommand, (optional "*",) params]
    let subcommand = msg.params.get(1).map(|s| s.as_str()).unwrap_or("");

    match subcommand {
        "LS" => {
            // Server listed its capabilities.
            // For now, request a small set of common useful caps.
            let want = ["multi-prefix", "away-notify", "server-time", "message-tags"];
            let server_caps = msg.params.last().map(|s| s.as_str()).unwrap_or("");

            let to_request: Vec<&str> = want
                .iter()
                .copied()
                .filter(|cap| server_caps.split_whitespace().any(|s| s == *cap))
                .collect();

            if to_request.is_empty() {
                sender.send(IrcMessage::new(Command::Cap, vec!["END".into()]));
                state.reg = RegistrationState::WaitingForWelcome;
            } else {
                sender.send(IrcMessage::new(
                    Command::Cap,
                    vec!["REQ".into(), to_request.join(" ")],
                ));
                state.reg = RegistrationState::CapPending;
            }
        }

        "ACK" => {
            // Server acknowledged our capability requests
            if let Some(caps) = msg.params.last() {
                for cap in caps.split_whitespace() {
                    state.caps.insert(cap.to_string());
                }
            }
            sender.send(IrcMessage::new(Command::Cap, vec!["END".into()]));
            state.reg = RegistrationState::WaitingForWelcome;
        }

        "NAK" => {
            // Server rejected our request — just end negotiation
            warn!("CAP NAK: {:?}", msg.params.last());
            sender.send(IrcMessage::new(Command::Cap, vec!["END".into()]));
            state.reg = RegistrationState::WaitingForWelcome;
        }

        other => {
            debug!("Unhandled CAP subcommand: {}", other);
        }
    }
}

/// Extract the nick from a message prefix, returning empty string if absent.
fn nick_from_prefix(prefix: &Option<Prefix>) -> String {
    match prefix {
        Some(Prefix::User { nick, .. }) => nick.clone(),
        Some(Prefix::Server(s)) => s.clone(),
        None => String::new(),
    }
}
