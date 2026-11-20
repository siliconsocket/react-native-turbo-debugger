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
    revisions: HashMap<(String, String), u64>,
}
impl Capture {
    pub fn new(session: Session) -> Self {
        let action = session
            .events
            .iter()
            .rev()
            .find(|e| matches!(e.data, Data::State { .. }))
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
            revisions: HashMap::new(),
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
                correlation_id,
            }) => {
                if self.session.events.iter().any(|e| e.id == id) {
                    return;
                }
                let captured_at = self.live_timestamp(at_ms);
                self.retain_history(false);
                self.pending.insert(id.clone(), at_ms);
                self.session.events.push(Event {
                    id,
                    at_ms: captured_at,
                    correlation_id,
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
            Update::Message(Message::State { at_ms, transition }) => {
                let StateTransition {
                    source,
                    store,
                    action,
                    before,
                    after,
                    payload,
                    correlation_id,
                } = *transition;
                let captured_at = self.live_timestamp(at_ms);
                self.retain_history(true);
                let revision = self
                    .revisions
                    .entry((source.clone(), store.clone()))
                    .or_default();
                self.sequence += 1;
                self.session.events.push(Event {
                    id: format!("state:{}", self.sequence),
                    at_ms: captured_at,
                    correlation_id,
                    data: Data::State {
                        source,
                        store,
                        action,
                        revision_before: *revision,
                        revision_after: *revision + 1,
                        before,
                        after,
                        payload,
                    },
                });
                *revision += 1;
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

    fn retain_history(&mut self, state: bool) {
        const RETAINED_PER_KIND: usize = 500;
        let mut matching = self
            .session
            .events
            .iter()
            .enumerate()
            .filter(|(_, event)| matches!(event.data, Data::State { .. }) == state);
        if let Some((oldest, _)) = matching.next()
            && matching.count() >= RETAINED_PER_KIND - 1
        {
            let removed = self.session.events.remove(oldest);
            self.pending.remove(&removed.id);
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
    pub fn visible(&self) -> impl DoubleEndedIterator<Item = &Event> {
        let cursor = self.cursor();
        self.session
            .events
            .iter()
            .filter(move |e| e.at_ms <= cursor)
    }
    pub fn visible_states(&self, enabled: [bool; 3]) -> impl DoubleEndedIterator<Item = &Event> {
        self.visible().filter(move |event| {
            let Data::State { source, .. } = &event.data else {
                return false;
            };
            STATE_SOURCES
                .iter()
                .zip(enabled)
                .all(|((key, _), on)| key != source || on)
        })
    }

    pub fn reconcile_action(&mut self, enabled: [bool; 3]) {
        if self
            .visible_states(enabled)
            .any(|event| Some(&event.id) == self.action.as_ref())
        {
            return;
        }
        let newest = self
            .visible_states(enabled)
            .next_back()
            .map(|event| event.id.clone());
        self.action = newest;
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
            correlation_id: None,
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
    fn state_revisions_and_filters_follow_the_session_and_cursor() {
        use serde_json::json;

        let mut c = Capture::live();
        let add = |c: &mut Capture, at_ms, source: &str, store: &str, correlation_id| {
            c.ingest(Update::Message(Message::State {
                at_ms,
                transition: Box::new(StateTransition {
                    source: source.into(),
                    store: store.into(),
                    action: "Increment".into(),
                    before: json!(0),
                    after: json!(1),
                    payload: None,
                    correlation_id,
                }),
            }));
        };
        add(&mut c, 100, "zustand", "counter", Some("trace".into()));
        c.ingest(Update::Message(Message::Start {
            id: "request".into(),
            at_ms: 101,
            method: "GET".into(),
            path: "localhost/counter".into(),
            correlation_id: Some("trace".into()),
        }));
        add(&mut c, 102, "redux", "counter", None);
        c.reconcile_action([true; 3]);
        assert_eq!(c.action.as_deref(), Some("state:2"));
        c.pin(c.newest());
        add(&mut c, 103, "jotai", "atom", None);
        add(&mut c, 104, "zustand", "counter", None);
        c.reconcile_action([true, false, true]);
        let selected = c.selected(&c.action).unwrap();
        assert_eq!(selected.id, "state:1");
        assert!(selected.related(&c.session.events[1]));
        assert!(!selected.related(&c.session.events[2]));
        c.reconcile_action([false, false, true]);
        assert!(c.action.is_none());
        c.pinned = None;
        c.reconcile_action([false, false, true]);
        assert_eq!(c.action.as_deref(), Some("state:3"));
        let revisions = |capture: &Capture| {
            capture
                .visible_states([true; 3])
                .map(|event| match &event.data {
                    Data::State {
                        revision_before,
                        revision_after,
                        ..
                    } => (*revision_before, *revision_after),
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(revisions(&c), [(0, 1), (0, 1), (0, 1), (1, 2)]);
        c.ingest(Update::Disconnected);
        assert_eq!(revisions(&c).len(), 4);
        c.ingest(Update::Message(Message::Hello {
            version: 1,
            app: "Demo".into(),
            device: "Emulator".into(),
            platform: "Android".into(),
        }));
        add(&mut c, 110, "zustand", "counter", None);
        assert_eq!(revisions(&c), [(0, 1)]);
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
