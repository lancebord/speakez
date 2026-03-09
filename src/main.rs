use clap::Parser;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent,
        KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ratatui::{Terminal, backend::CrosstermBackend};
use scanpw::scanpw;
use std::io;
use std::net::ToSocketAddrs;
use tokio::sync::mpsc;

use irc_client::client::{Client, Config};
use irc_client::proto::message::{Command, IrcMessage};
use irc_client::{client::event::Event as IrcEvent, connection::Sender};
use tui::app::AppState;
use tui::ui;
mod tui;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    server: String,

    #[arg(short, long)]
    nick: String,

    #[arg(short, long, default_value_t = String::from("speakez"))]
    user: String,

    #[arg(short, long, default_value_t = String::from("speakez"))]
    realname: String,
}

enum AppEvent {
    Key(KeyEvent),
    Irc(IrcEvent),
    Resize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let password = scanpw!("Password (leave blank for no pass): ");

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = run(&mut terminal, password).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    result
}

async fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    password: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = AppState::new();

    let args = Args::try_parse()?;
    if args.server.to_socket_addrs().is_err() {
        return Err("Error: could not resolve server".into());
    }

    let password = match password.as_str() {
        "" => None,
        _ => Some(password),
    };

    // Connect to IRC
    let config = Config {
        server: args.server,
        nick: args.nick,
        user: args.user,
        realname: args.realname,
        password,
    };

    let mut client = Client::connect(config).await?;
    let sender = client.sender();

    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();
    // Spawn keyboard task — blocks on crossterm's async read,
    // zero CPU until a key is actually pressed

    let kb_tx = tx.clone();
    tokio::spawn(async move {
        loop {
            // event::EventStream is crossterm's async Stream adapter
            match EventStream::new().next().await {
                Some(Ok(Event::Key(key))) => {
                    if kb_tx.send(AppEvent::Key(key)).is_err() {
                        break;
                    }
                }
                Some(Ok(Event::Resize(_, _))) => {
                    if kb_tx.send(AppEvent::Resize).is_err() {
                        break;
                    }
                }
                None => break,
                _ => {}
            }
        }
    });

    // Spawn IRC task — awaits silently until the server sends something
    let irc_tx = tx.clone();
    tokio::spawn(async move {
        while let Some(event) = client.next_event().await {
            if irc_tx.send(AppEvent::Irc(event)).is_err() {
                break;
            }
        }
    });

    // Draw once at startup
    terminal.draw(|f| ui::draw(f, &mut app))?;
    // We poll both IRC events and keyboard events with short timeouts so
    // neither blocks the other.
    // Main loop: sleeps until an event arrives, redraws only on state change
    while let Some(event) = rx.recv().await {
        let mut dirty = true;

        match event {
            AppEvent::Key(key) => {
                match (key.modifiers, key.code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => break,
                    (_, KeyCode::Enter) => {
                        let text = app.take_input();
                        if !text.is_empty() {
                            // need client here — see note below about Arc<Mutex<Client>>
                            if handle_input(&text, &mut app, &sender) {
                                break;
                            }
                        }
                    }
                    (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                        app.input_insert(c);
                    }
                    (KeyModifiers::SHIFT, KeyCode::Up) => app.members_scroll_up(),
                    (KeyModifiers::SHIFT, KeyCode::Down) => app.members_scroll_down(),
                    (_, KeyCode::Backspace) => app.input_backspace(),
                    (_, KeyCode::Left) => app.cursor_left(),
                    (_, KeyCode::Right) => app.cursor_right(),
                    (_, KeyCode::Up) => app.scroll_up(),
                    (_, KeyCode::Down) => app.scroll_down(),
                    (_, KeyCode::Home) => app.cursor = 0,
                    (_, KeyCode::End) => app.cursor = app.input.len(),
                    _ => {
                        dirty = false;
                    }
                }
            }

            AppEvent::Irc(irc_event) => {
                match &irc_event {
                    IrcEvent::Raw(_) => dirty = false,
                    _ => {}
                }
                handle_irc_event(irc_event, &mut app);
            }

            AppEvent::Resize => {} // just needs to be handled so dirty is true
        }

        if dirty {
            terminal.draw(|f| ui::draw(f, &mut app))?;
        }
    }

    Ok(())
}

/// Handle a line entered in the input box.
fn handle_input(text: &str, app: &mut AppState, sender: &Sender) -> bool {
    if let Some(cmd) = text.strip_prefix('/') {
        // It's a command
        let mut parts = cmd.splitn(2, ' ');
        let verb = parts.next().unwrap_or("").to_uppercase();
        let args = parts.next().unwrap_or("");

        match verb.as_str() {
            "JOIN" => {
                if !app.channel.is_empty() {
                    sender.part(&app.channel, None);
                }
                app.messages.clear();
                app.members.clear();
                sender.join(args.trim());
                app.channel = args.trim().to_string();
            }
            "PART" => {
                let channel = if args.is_empty() {
                    &app.channel
                } else {
                    args.trim()
                };
                sender.part(channel, None);
                app.channel = "".to_string();
                app.members.clear();
            }
            "NICK" => {
                sender.nick(args.trim());
            }
            "QUIT" => {
                sender.send(IrcMessage::new(Command::Quit, vec![args.to_string()]));
                return true;
            }
            "ME" => {
                // CTCP ACTION
                let ctcp = format!("\x01ACTION {}\x01", args);
                sender.privmsg(&app.channel, &ctcp);
                app.push_system(&format!("* {} {}", app.nick, args));
            }
            "MSG" => {
                let mut p = args.splitn(2, ' ');
                if let (Some(target), Some(msg)) = (p.next(), p.next()) {
                    sender.privmsg(target, msg);
                    app.push_message(&format!("You →  {target}:"), &msg);
                }
            }
            other => {
                app.push_system(&format!("Unknown command: /{}", other));
            }
        }
    } else {
        if app.connected && !app.channel.is_empty() {
            // Regular chat message to active channel
            sender.privmsg(&app.channel, text);
            app.push_message(&app.nick.clone(), text);
        }
    }
    false
}

/// Apply an IRC event to the app state.
fn handle_irc_event(event: IrcEvent, app: &mut AppState) {
    match event {
        IrcEvent::Connected { server, nick } => {
            app.nick = nick.clone();
            app.connected = true;
            app.status = format!("connected to {}", server);
            app.push_system(&format!("Connected to {} as {}", server, nick));
        }

        IrcEvent::Joined { channel, nick } => {
            if nick == app.nick {
                app.push_system(&format!("You joined {}", channel));
            } else {
                app.push_system(&format!("{} joined {}", nick, channel));
                app.members.push(nick);
                app.sort_members();
            }
        }

        IrcEvent::Message {
            from,
            target,
            text,
            is_notice: _,
        } => {
            // Only show messages for our active channel (or PMs to us)
            let is_self = from == app.nick;
            if !is_self {
                // Don't re-echo our own messages (we already pushed them in handle_input)
                if target == app.channel {
                    app.push_message(&from, &text);
                } else if target == app.nick {
                    app.push_message(&format!("{from} →  You:"), &text);
                }
            }
        }

        IrcEvent::SysMessage { text } => {
            app.push_system(text.as_str());
        }

        IrcEvent::Parted {
            channel,
            nick,
            reason,
        } => {
            app.members.retain(|m| {
                let bare = m.trim_start_matches(&['@', '+', '%'][..]);
                bare != nick
            });
            app.push_system(&format!(
                "{} left {}{}",
                nick,
                channel,
                reason.map(|r| format!(" ({})", r)).unwrap_or_default()
            ));
        }

        IrcEvent::Quit { nick, reason } => {
            app.members.retain(|m| {
                let bare = m.trim_start_matches(&['@', '+', '%'][..]);
                bare != nick
            });
            app.push_system(&format!(
                "{} quit{}",
                nick,
                reason.map(|r| format!(" ({})", r)).unwrap_or_default()
            ));
        }

        IrcEvent::NickChanged { old_nick, new_nick } => {
            for m in &mut app.members {
                let bare = m.trim_start_matches(&['@', '+', '%'][..]).to_string();
                if bare == old_nick {
                    let sigil: String = m.chars().take_while(|c| "@+%~&".contains(*c)).collect();
                    *m = format!("{}{}", sigil, new_nick);
                    break;
                }
            }
            if old_nick == app.nick {
                app.nick = new_nick.clone();
            }
            app.push_system(&format!("{} is now {}", old_nick, new_nick));
        }

        IrcEvent::Topic { channel, topic } => {
            app.status = format!("{}: {}", channel, topic);
            app.push_system(&format!("Topic: {}", topic));
        }

        IrcEvent::Names {
            channel: _,
            members,
        } => {
            for m in members {
                if !app
                    .members
                    .iter()
                    .any(|existing| existing.trim_start_matches(&['@', '+', '%'][..]) == m)
                {
                    app.members.push(m);
                }
            }
            app.sort_members();
        }

        IrcEvent::Disconnected => {
            app.connected = false;
            app.status = "disconnected".to_string();
            app.push_system("--- Disconnected ---");
        }

        IrcEvent::Raw(_) => {}
    }
}
