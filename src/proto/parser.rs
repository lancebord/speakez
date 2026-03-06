use crate::proto::error::ParseError;
use crate::proto::message::{Command, IrcMessage, Prefix};
use std::collections::HashMap;

pub fn parse(line: &str) -> Result<IrcMessage, ParseError> {
    if line.is_empty() {
        return Err(ParseError::EmptyMessage);
    }

    let mut rest = line;

    // parse tags
    let tags = if rest.starts_with('@') {
        let (tag_str, remaining) = rest[1..]
            .split_once(' ')
            .ok_or(ParseError::MissingCommand)?;
        rest = remaining;
        parse_tags(tag_str)?
    } else {
        HashMap::new()
    };

    // parse prefix
    let prefix = if rest.starts_with(':') {
        let (prefix_str, remaining) = rest[1..]
            .split_once(' ')
            .ok_or(ParseError::MissingCommand)?;
        rest = remaining;
        Some(parse_prefix(prefix_str))
    } else {
        None
    };

    // parse command
    let (command_str, rest) = match rest.split_once(' ') {
        Some((cmd, params)) => (cmd, params),
        None => (rest, ""),
    };

    if command_str.is_empty() {
        return Err(ParseError::MissingCommand);
    }

    let command = Command::from_str(command_str);

    // parse params
    let params = parse_params(rest);

    Ok(IrcMessage {
        tags,
        prefix,
        command,
        params,
    })
}

fn parse_tags(tag_str: &str) -> Result<HashMap<String, Option<String>>, ParseError> {
    let mut tags = HashMap::new();

    for tag in tag_str.split(';') {
        if tag.is_empty() {
            continue;
        }
        match tag.split_once('=') {
            Some((key, value)) => {
                tags.insert(key.to_string(), Some(unescape_tag_value(value)));
            }
            None => {
                // boolean tags
                tags.insert(tag.to_string(), None);
            }
        }
    }

    Ok(tags)
}

fn unescape_tag_value(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some(':') => result.push(';'),
                Some('s') => result.push(' '),
                Some('\\') => result.push('\\'),
                Some('r') => result.push('\r'),
                Some('n') => result.push('\n'),
                Some(c) => {
                    result.push('\\');
                    result.push(c);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }

    result
}

fn parse_prefix(prefix: &str) -> Prefix {
    if prefix.contains('!') || prefix.contains('@') {
        let (nick, rest) = prefix
            .split_once('!')
            .map(|(n, r)| (n, Some(r)))
            .unwrap_or((prefix, None));

        let (user, host) = match rest {
            Some(r) => r
                .split_once('@')
                .map(|(u, h)| (Some(u.to_string()), Some(h.to_string())))
                .unwrap_or((Some(r.to_string()), None)),
            None => {
                // Could be nick@host with no user
                if let Some((n2, h)) = nick.split_once('@') {
                    return Prefix::User {
                        nick: n2.to_string(),
                        user: None,
                        host: Some(h.to_string()),
                    };
                }
                (None, None)
            }
        };

        Prefix::User {
            nick: nick.to_string(),
            user,
            host,
        }
    } else {
        // Heuristic: if it contains a dot, it's likely a server name
        // (nick-only prefixes are also possible but rare without user/host)
        Prefix::Server(prefix.to_string())
    }
}

fn parse_params(params_str: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut rest = params_str;

    loop {
        rest = rest.trim_start_matches(' ');

        if rest.is_empty() {
            break;
        }

        if rest.starts_with(':') {
            params.push(rest[1..].to_string());
            break;
        }

        match rest.split_once(' ') {
            Some((param, remaining)) => {
                params.push(param.to_string());
                rest = remaining;
            }
            None => {
                params.push(rest.to_string());
                break;
            }
        }
    }

    params
}
