//! The local Debug collector receives length-prefixed UTF-8 JSON from Debug builds.

use crate::session::{NetworkOutcome, StateTransition};
use serde::Deserialize;
use serde_json::value::RawValue;
use std::{
    io::{self, Read},
    net::{Ipv4Addr, TcpListener, TcpStream},
    sync::mpsc::{self, Receiver, SyncSender},
    time::Duration,
};

pub const PORT: u16 = 7410;
const MAX_TRANSITION: usize = 16 * 1024;
// The state frame also contains its type and native monotonic timestamp.
const MAX_FRAME: usize = MAX_TRANSITION + 64;

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
        correlation_id: Option<String>,
    },
    State {
        at_ms: u64,
        transition: Box<StateTransition>,
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
        .name("debugger-bridge".into())
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
    let Ok(Some(hello @ Message::Hello { version: 1, .. })) = read_message(&*stream) else {
        return true;
    };
    if stream.set_read_timeout(None).is_err() {
        return true;
    }
    if send.send(Update::Message(hello)).is_err() {
        return false;
    }
    while let Ok(message) = read_message(&*stream) {
        match message {
            Some(Message::Hello { .. }) => break,
            Some(message) => {
                if send.send(Update::Message(message)).is_err() {
                    return false;
                }
            }
            None => continue,
        }
    }
    send.send(Update::Disconnected).is_ok()
}

fn read_message(mut input: impl Read) -> io::Result<Option<Message>> {
    let mut prefix = [0; 4];
    input.read_exact(&mut prefix)?;
    let length = u32::from_be_bytes(prefix) as usize;
    if length == 0 || length > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid debugger frame size",
        ));
    }
    let mut payload = vec![0; length];
    input.read_exact(&mut payload)?;
    let Ok(message) = serde_json::from_slice(&payload) else {
        return Ok(None);
    };
    if let Message::State { transition, .. } = &message {
        if [&transition.source, &transition.store, &transition.action]
            .iter()
            .any(|value| value.trim().is_empty())
        {
            return Ok(None);
        }
        if length > MAX_TRANSITION {
            #[derive(Deserialize)]
            struct StateFrame<'a> {
                #[serde(borrow)]
                transition: &'a RawValue,
            }
            if !serde_json::from_slice::<StateFrame>(&payload)
                .is_ok_and(|frame| frame.transition.get().len() <= MAX_TRANSITION)
            {
                return Ok(None);
            }
        }
    } else if length > MAX_TRANSITION {
        return Ok(None);
    }
    Ok(Some(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(json: &[u8]) -> Vec<u8> {
        let mut frame = (json.len() as u32).to_be_bytes().to_vec();
        frame.extend_from_slice(json);
        frame
    }

    #[test]
    fn state_frames_respect_the_utf8_limit_and_preserve_the_stream() {
        let transition =
            r#"{"source":"jotai","store":"counter","action":"Set","before":null,"after":""}"#;
        let sized = |extra| {
            let mut body = transition.to_owned();
            let padding = MAX_TRANSITION - body.len() + extra;
            let filler = "é".repeat(padding / 2) + &"x".repeat(padding % 2);
            body.insert_str(body.len() - 2, &filler);
            frame(format!(r#"{{"type":"state","atMs":1,"transition":{body}}}"#).as_bytes())
        };
        let frames = [
            frame(b"{invalid"),
            frame(br#"{"type":"state","atMs":1,"transition":{"source":"jotai","store":"counter","action":"Set","after":0}}"#),
            sized(1),
            sized(0),
            frame(br#"{"type":"start","id":"1","atMs":2,"method":"GET","path":"localhost/demo"}"#),
        ].concat();
        let mut input = frames.as_slice();
        for _ in 0..3 {
            assert!(read_message(&mut input).unwrap().is_none());
        }
        let Some(Message::State {
            at_ms: 1,
            transition,
        }) = read_message(&mut input).unwrap()
        else {
            panic!("expected a valid state frame at the limit");
        };
        assert!(transition.before.is_null());
        assert!(matches!(
            read_message(&mut input).unwrap(),
            Some(Message::Start {
                correlation_id: None,
                ..
            })
        ));
        assert!(input.is_empty());
    }

    #[test]
    fn reads_mobile_frames_and_rejects_oversized_payload_before_allocating() {
        let json = br#"{"type":"finish","id":"1","atMs":51,"status":503,"outcome":"response"}"#;
        let framed = frame(json);
        assert!(matches!(
            read_message(&framed[..]),
            Ok(Some(Message::Finish {
                at_ms: 51,
                status: Some(503),
                ..
            }))
        ));
        assert_eq!(
            read_message(&((MAX_FRAME + 1) as u32).to_be_bytes()[..])
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
}
