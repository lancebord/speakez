use bytes::{BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::proto::error::CodecError;
use crate::proto::message::IrcMessage;
use crate::proto::parser::parse;
use crate::proto::serializer::serialize;

const MAX_LINE_LENGTH: usize = 512;

pub struct IrcCodec {
    max_line_length: usize,
}

impl IrcCodec {
    pub fn new() -> Self {
        Self {
            max_line_length: MAX_LINE_LENGTH,
        }
    }

    pub fn with_max_length(max_line_length: usize) -> Self {
        Self { max_line_length }
    }
}

impl Default for IrcCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for IrcCodec {
    type Item = IrcMessage;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            let newline_pos = src.iter().position(|&b| b == b'\n');

            match newline_pos {
                None => {
                    if src.len() > self.max_line_length {
                        return Err(CodecError::Parse(
                            crate::proto::error::ParseError::MessageTooLong,
                        ));
                    }
                    return Ok(None);
                }
                Some(pos) => {
                    let line_bytes = src.split_to(pos + 1);

                    let line = &line_bytes[..line_bytes.len() - 1]; // strip \n
                    let line = if line.last() == Some(&b'\r') {
                        &line[..line.len() - 1] // strip \r
                    } else {
                        line
                    };

                    // Skip empty lines silently
                    if line.is_empty() {
                        continue;
                    }

                    let line_str = std::str::from_utf8(line).map_err(|_| {
                        CodecError::Io(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "IRC message is not valid UTF-8",
                        ))
                    })?;

                    let msg = parse(line_str)?;
                    return Ok(Some(msg));
                }
            }
        }
    }
}

impl Encoder<IrcMessage> for IrcCodec {
    type Error = CodecError;

    fn encode(&mut self, msg: IrcMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let line = serialize(&msg);

        // +2 for \r\n
        if line.len() + 2 > self.max_line_length {
            return Err(CodecError::Parse(
                crate::proto::error::ParseError::MessageTooLong,
            ));
        }

        dst.reserve(line.len() + 2);
        dst.put_slice(line.as_bytes());
        dst.put_slice(b"\r\n");
        Ok(())
    }
}
