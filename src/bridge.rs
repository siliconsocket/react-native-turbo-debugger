//! The local network collector receives length-prefixed UTF-8 JSON from Debug builds.

use crate::session::NetworkOutcome;
use serde::Deserialize;
use std::{
    io::{self, Read},
    net::{Ipv4Addr, TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, SyncSender},
    time::Duration,
};

pub const PORT: u16 = 7410;
const MAX_FRAME: usize = 16 * 1024;

#[derive(Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum Message {
    Hello {
        version: u8,
        app: String,
        device: String,
        platform: String,
    },
    Start {
        id: String,
        at_ms: u64,
        method: String,
        path: String,
    },
    Finish {
        id: String,
        at_ms: u64,
        status: Option<u16>,
        outcome: NetworkOutcome,
    },
}

pub enum Update {
    Message(Message),
    Disconnected,
}

pub fn listen() -> io::Result<Receiver<Update>> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, PORT))?;
    let (send, receive) = mpsc::sync_channel(256);
    std::thread::Builder::new()
        .name("network-bridge".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                if !receive_session(&mut stream, &send) {
                    break;
                }
            }
        })?;
    Ok(receive)
}

// Accept one foreground app at a time. A failed handshake never changes the active session.
fn receive_session(stream: &mut TcpStream, send: &SyncSender<Update>) -> bool {
    if stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .is_err()
    {
        return true;
    }
    let Ok(hello @ Message::Hello { version: 1, .. }) = read_message(&*stream) else {
        return true;
    };
    if stream.set_read_timeout(None).is_err() {
        return true;
    }
    if send.send(Update::Message(hello)).is_err() {
        return false;
    }
    while let Ok(message @ (Message::Start { .. } | Message::Finish { .. })) =
        read_message(&*stream)
    {
        if send.send(Update::Message(message)).is_err() {
            return false;
        }
    }
    send.send(Update::Disconnected).is_ok()
}

fn read_message(mut input: impl Read) -> io::Result<Message> {
    let mut prefix = [0; 4];
    input.read_exact(&mut prefix)?;
    let length = u32::from_be_bytes(prefix) as usize;
    if length == 0 || length > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid network frame size",
        ));
    }
    let mut payload = vec![0; length];
    input.read_exact(&mut payload)?;
    serde_json::from_slice(&payload).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_mobile_frames_and_rejects_oversized_payload_before_allocating() {
        let json = br#"{"type":"finish","id":"1","atMs":51,"status":503,"outcome":"response"}"#;
        let mut framed = (json.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(json);
        assert!(matches!(
            read_message(&framed[..]),
            Ok(Message::Finish {
                at_ms: 51,
                status: Some(503),
                ..
            })
        ));
        assert_eq!(
            read_message(&((MAX_FRAME + 1) as u32).to_be_bytes()[..])
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
}
