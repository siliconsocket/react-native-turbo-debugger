use crate::session::*;
use serde_json::json;

pub fn fixture() -> Session {
    serde_json::from_str(include_str!("../assets/fixture.json")).expect("valid bundled session")
}

pub fn scenario(name: &str) -> Session {
    let mut session = fixture();
    if name == "empty" {
        session.events.clear();
    }
    for event in &mut session.events {
        match (name, &mut event.data) {
            (
                "pending",
                Data::Network {
                    path,
                    status,
                    duration_ms,
                    response,
                    ..
                },
            ) if path == "/api/sync" => {
                *status = None;
                *duration_ms = None;
                *response = None;
            }
            ("long", Data::State { after, .. }) => {
                after["description"] = "Mountain trail ".repeat(100).into();
            }
            _ => {}
        }
    }
    session
}

pub fn next_event(sequence: usize, at_ms: u64) -> Event {
    let revision = 5 + sequence as u64;
    Event {
        id: format!("demo-{sequence}"),
        correlation_id: None,
        at_ms,
        data: Data::State {
            source: "zustand".into(),
            store: "useTrailStore".into(),
            action: "updateViewport".into(),
            revision_before: revision,
            revision_after: revision + 1,
            before: json!({"zoom": revision}),
            after: json!({"zoom": revision + 1}),
            payload: Some(json!({"zoom": revision + 1})),
        },
    }
}
