use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ParseError {
    #[error("message is empty")]
    EmptyMessage,

    #[error("missing command")]
    MissingCommand,

    #[error("invalid tag format: {0}")]
    InvalidTag(String),

    #[error("line exceeds max message length")]
    MessageTooLong,
}

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
