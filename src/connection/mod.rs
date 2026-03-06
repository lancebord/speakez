use futures::SinkExt;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use tracing::{debug, error, info};

use crate::proto::codec::IrcCodec;
use crate::proto::message::IrcMessage;

/// A handle to send messages to the server.
/// Cheaply cloneable — pass it wherever you need to write.
#[derive(Clone)]
pub struct Sender {
    tx: mpsc::UnboundedSender<IrcMessage>,
}

impl Sender {
    pub fn send(&self, msg: IrcMessage) {
        // Only fails if the connection task has shut down
        let _ = self.tx.send(msg);
    }
}

/// Establish a TCP connection and return:
/// - A `Sender` handle for writing messages
/// - An `mpsc::UnboundedReceiver<IrcMessage>` for reading incoming messages
///
/// Two background tasks are spawned:
/// - A **writer task**: drains the sender channel and writes to the TCP stream
/// - A **reader task**: reads from the TCP stream and forwards to the inbox
///
/// This split means the caller never has to hold a lock to send a message.
pub async fn connect(
    addr: &str,
) -> Result<(Sender, mpsc::UnboundedReceiver<IrcMessage>), std::io::Error> {
    info!("Connecting to {}", addr);
    let stream = TcpStream::connect(addr).await?;
    info!("TCP connected to {}", addr);

    let framed = Framed::new(stream, IrcCodec::new());
    let (mut sink, mut stream) = futures::StreamExt::split(framed);

    // Channel for outbound messages (caller → writer task)
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<IrcMessage>();

    // Channel for inbound messages (reader task → caller)
    let (in_tx, in_rx) = mpsc::unbounded_channel::<IrcMessage>();

    // Writer task: takes messages from out_rx and sends them to the server
    tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            debug!("--> {}", crate::proto::serializer::serialize(&msg));
            if let Err(e) = sink.send(msg).await {
                error!("Write error: {}", e);
                break;
            }
        }
        debug!("Writer task shut down");
    });

    // Reader task: receives messages from the server and forwards to in_tx
    tokio::spawn(async move {
        use futures::StreamExt;
        while let Some(result) = stream.next().await {
            match result {
                Ok(msg) => {
                    debug!("<-- {}", crate::proto::serializer::serialize(&msg));
                    if in_tx.send(msg).is_err() {
                        break; // Receiver dropped, shut down
                    }
                }
                Err(e) => {
                    error!("Read error: {}", e);
                    break;
                }
            }
        }
        debug!("Reader task shut down");
    });

    Ok((Sender { tx: out_tx }, in_rx))
}
