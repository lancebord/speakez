use crate::proto::message::{IrcMessage, Prefix};
use std::fmt::Write;

pub fn serialize(msg: &IrcMessage) -> String {
    let mut out = String::with_capacity(512);

    // tags
    if !msg.tags.is_empty() {
        out.push('@');
        let mut first = true;
        for (key, value) in &msg.tags {
            if !first {
                out.push(';');
            }
            first = false;
            out.push_str(key);
            if let Some(v) = value {
                out.push('=');
                escape_tag_value(&mut out, v);
            }
        }
        out.push(' ');
    }

    // prefix
    if let Some(prefix) = &msg.prefix {
        out.push(':');
        match prefix {
            Prefix::Server(s) => out.push_str(s),
            Prefix::User { nick, user, host } => {
                out.push_str(nick);
                if let Some(u) = user {
                    out.push('!');
                    out.push_str(u);
                }
                if let Some(h) = host {
                    out.push('@');
                    out.push_str(h);
                }
            }
        }
        out.push(' ');
    }

    // command
    let _ = write!(out, "{}", msg.command);

    // params
    let last_idx = msg.params.len().saturating_sub(1);
    for (i, param) in msg.params.iter().enumerate() {
        out.push(' ');
        // The last param must be trailing if it contains spaces or starts with ':'
        let needs_trailing = i == last_idx
            && (param.contains(' ')
                || param.starts_with(':')
                || param.is_empty()
                || msg.params.len() > 1);
        if needs_trailing {
            out.push(':');
        }
        out.push_str(param);
    }

    out
}

fn escape_tag_value(out: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            ';' => out.push_str(r"\:"),
            ' ' => out.push_str(r"\s"),
            '\\' => out.push_str(r"\\"),
            '\r' => out.push_str(r"\r"),
            '\n' => out.push_str(r"\n"),
            c => out.push(c),
        }
    }
}
