use crate::{mock, session::*};

pub struct Capture {
    pub session: Session,
    pub pinned: Option<u64>,
    pub stopped: bool,
    pub connected: bool,
    pub action: Option<String>,
    pub request: Option<String>,
    sequence: usize,
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
        }
    }
    pub fn newest(&self) -> u64 {
        self.session
            .events
            .iter()
            .map(|e| e.at_ms)
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
