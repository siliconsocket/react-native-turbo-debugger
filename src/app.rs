use crate::{
    bridge::{Message, Update},
    mock,
    session::*,
};
use std::collections::HashMap;

pub struct Capture {
    pub session: Session,
    pub pinned: Option<u64>,
    pub stopped: bool,
    pub connected: bool,
    pub action: Option<String>,
    pub request: Option<String>,
    sequence: usize,
    live_origin: Option<u64>,
    pending: HashMap<String, u64>,
}
impl Capture {
    pub fn new(session: Session) -> Self {
        let action = session
            .events
            .iter()
            .rev()
            .find(|e| matches!(e.data, Data::Zustand { .. }))
            .map(|e| e.id.clone());
        Self {
            session,
            pinned: None,
            stopped: false,
            connected: true,
            action,
            request: None,
            sequence: 0,
            live_origin: None,
            pending: HashMap::new(),
        }
    }
    pub fn live() -> Self {
        let mut capture = Self::new(Session {
            session_id: "live".into(),
            app: "React Native".into(),
            app_icon: None,
            device: "Waiting for app".into(),
            events: Vec::new(),
        });
        capture.connected = false;
        capture
    }

    pub fn ingest(&mut self, update: Update) {
        if self.stopped {
            return;
        }
        match update {
            Update::Message(Message::Hello {
                app,
                device,
                platform,
                ..
            }) => {
                *self = Self::live();
                self.session.app = app;
                self.session.device = format!("{device} · {platform}");
                self.connected = true;
            }
            Update::Message(Message::Start {
                id,
                at_ms,
                method,
                path,
            }) => {
                if self.session.events.iter().any(|e| e.id == id) {
                    return;
                }
                const RETAINED_REQUESTS: usize = 500;
                if self.session.events.len() >= RETAINED_REQUESTS {
                    let oldest = self.session.events.remove(0);
                    self.pending.remove(&oldest.id);
                }
                let captured_at = self.live_timestamp(at_ms);
                self.pending.insert(id.clone(), at_ms);
                self.session.events.push(Event {
                    id,
                    at_ms: captured_at,
                    correlation_id: None,
                    data: Data::Network {
                        method,
                        path,
                        status: None,
                        duration_ms: None,
                        response: None,
                        result: None,
                    },
                });
            }
            Update::Message(Message::Finish {
                id,
                at_ms,
                status,
                outcome,
            }) => {
                if let Some(started) = self.pending.remove(&id) {
                    let finished = self.live_timestamp(at_ms);
                    self.finish(
                        &id,
                        finished,
                        status,
                        Some(at_ms.saturating_sub(started)),
                        outcome,
                    );
                }
            }
            Update::Disconnected => {
                self.connected = false;
                let at_ms = self.newest().saturating_add(1);
                let pending = std::mem::take(&mut self.pending);
                for id in pending.into_keys() {
                    self.finish(&id, at_ms, None, None, NetworkOutcome::Disconnected);
                }
            }
        }
    }

    fn live_timestamp(&mut self, at_ms: u64) -> u64 {
        let origin = *self.live_origin.get_or_insert(at_ms);
        let elapsed = at_ms.saturating_sub(origin);
        if self.session.events.is_empty() {
            elapsed
        } else {
            elapsed.max(self.newest().saturating_add(1))
        }
    }

    fn finish(
        &mut self,
        id: &str,
        at_ms: u64,
        status: Option<u16>,
        duration: Option<u64>,
        outcome: NetworkOutcome,
    ) {
        if let Some(Event {
            data:
                Data::Network {
                    status: code,
                    duration_ms,
                    result,
                    ..
                },
            ..
        }) = self.session.events.iter_mut().find(|e| e.id == id)
        {
            *code = status;
            *duration_ms = duration;
            *result = Some(NetworkResult { at_ms, outcome });
        }
    }

    pub fn newest(&self) -> u64 {
        self.session
            .events
            .iter()
            .map(|e| match &e.data {
                Data::Network {
                    result: Some(result),
                    ..
                } => result.at_ms,
                _ => e.at_ms,
            })
            .max()
            .unwrap_or(0)
    }
    pub fn cursor(&self) -> u64 {
        self.pinned.unwrap_or_else(|| self.newest())
    }
    pub fn visible(&self) -> impl Iterator<Item = &Event> {
        let cursor = self.cursor();
        self.session
            .events
            .iter()
            .filter(move |e| e.at_ms <= cursor)
    }
    pub fn selected(&self, id: &Option<String>) -> Option<&Event> {
        self.visible().find(|e| Some(&e.id) == id.as_ref())
    }
    pub fn pin(&mut self, t: u64) {
        self.pinned = Some(t.min(self.newest()));
    }
    pub fn append(&mut self) {
        if self.stopped || !self.connected {
            return;
        }
        self.session
            .events
            .push(mock::next_event(self.sequence, self.newest() + 193));
        self.sequence += 1;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn correlation_and_fixture() {
        let c = Capture::new(mock::fixture());
        let a = c.selected(&c.action).unwrap();
        assert_eq!(
            c.visible()
                .filter(|e| a.related(e))
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            ["j1", "n1", "r1"]
        );
        let orphan = mock::next_event(0, 900);
        assert!(!orphan.related(&mock::next_event(1, 1000)));
    }
    #[test]
    fn failed_sync_links_to_retry_warning() {
        let c = Capture::new(mock::fixture());
        let sync = c
            .visible()
            .find(|e| matches!(&e.data, Data::Network{path,..} if path=="/api/sync"))
            .unwrap();
        assert!(matches!(
            sync.data,
            Data::Network {
                status: Some(503),
                duration_ms: Some(212),
                ..
            }
        ));
        let related: Vec<_> = c.visible().filter(|e| sync.related(e)).collect();
        assert_eq!(related.len(), 1);
        let warning = related[0].log(false).unwrap();
        assert_eq!(warning.level, "WARN");
        assert_eq!(warning.args[0]["retryInMs"], 5000);
    }
    #[test]
    fn live_request_updates_preserve_the_pinned_inspection() {
        let mut capture = Capture::live();
        capture.ingest(Update::Message(Message::Hello {
            version: 1,
            app: "Demo".into(),
            device: "Simulator".into(),
            platform: "iOS".into(),
        }));
        capture.ingest(Update::Message(Message::Start {
            id: "1".into(),
            at_ms: 100,
            method: "GET".into(),
            path: "localhost/demo".into(),
        }));
        capture.request = Some("1".into());
        capture.pin(capture.newest());
        capture.ingest(Update::Message(Message::Finish {
            id: "1".into(),
            at_ms: 160,
            status: Some(200),
            outcome: NetworkOutcome::Response,
        }));
        let status = |c: &Capture| {
            c.selected(&c.request)
                .unwrap()
                .network_status_at(c.cursor())
        };
        assert_eq!(status(&capture), Some((None, None, None)));
        capture.pinned = None;
        assert_eq!(
            status(&capture),
            Some((Some(200), Some(60), Some(NetworkOutcome::Response)))
        );
        capture.ingest(Update::Disconnected);
        assert!(!capture.connected);
        assert_eq!(capture.session.events.len(), 1);
    }

    #[test]
    fn cursor_and_selection_survive_arrivals() {
        let mut c = Capture::new(mock::fixture());
        c.pin(c.newest());
        c.append();
        assert_eq!(c.visible().count(), 6);
        c.pinned = None;
        assert_eq!(c.visible().count(), 7);
        assert_eq!(c.selected(&c.action).unwrap().id, "a1");
        c.stopped = true;
        c.append();
        assert_eq!(c.visible().count(), 7);
    }
}
