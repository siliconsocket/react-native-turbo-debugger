use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub session_id: String,
    pub app: String,
    pub app_icon: Option<String>,
    pub device: String,
    pub events: Vec<Event>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub at_ms: u64,
    pub correlation_id: Option<String>,
    #[serde(flatten)]
    pub data: Data,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum Data {
    State {
        source: String,
        store: String,
        action: String,
        revision_before: u64,
        revision_after: u64,
        before: Value,
        after: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload: Option<Value>,
    },
    Network {
        method: String,
        path: String,
        status: Option<u16>,
        duration_ms: Option<u64>,
        response: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        result: Option<NetworkResult>,
    },
    Js(Log),
    Native(Log),
}
pub const STATE_SOURCES: [(&str, &str); 3] = [
    ("zustand", "Zustand"),
    ("redux", "Redux"),
    ("jotai", "Jotai"),
];

pub fn state_source_label(source: &str) -> &str {
    STATE_SOURCES
        .iter()
        .find_map(|(key, label)| (*key == source).then_some(*label))
        .unwrap_or(source)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateTransition {
    pub source: String,
    pub store: String,
    pub action: String,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    pub before: Value,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    pub after: Value,
    #[serde(default, deserialize_with = "explicit_payload")]
    pub payload: Option<Value>,
    pub correlation_id: Option<String>,
}

fn explicit_payload<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(d).map(Some)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkOutcome {
    Response,
    Failed,
    Cancelled,
    Disconnected,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkResult {
    pub at_ms: u64,
    pub outcome: NetworkOutcome,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Log {
    pub level: String,
    pub message: String,
    pub subsystem: Option<String>,
    #[serde(default)]
    pub args: Vec<Value>,
}

impl Event {
    pub fn network_status_at(
        &self,
        cursor: u64,
    ) -> Option<(Option<u16>, Option<u64>, Option<NetworkOutcome>)> {
        let Data::Network {
            status,
            duration_ms,
            result,
            ..
        } = &self.data
        else {
            return None;
        };
        Some(match result {
            Some(result) if result.at_ms > cursor => (None, None, None),
            Some(result) => (*status, *duration_ms, Some(result.outcome)),
            None => (*status, *duration_ms, None),
        })
    }

    pub fn log(&self, native: bool) -> Option<&Log> {
        match (&self.data, native) {
            (Data::Js(log), false) | (Data::Native(log), true) => Some(log),
            _ => None,
        }
    }
    pub fn related(&self, other: &Self) -> bool {
        self.id != other.id
            && self
                .correlation_id
                .as_ref()
                .is_some_and(|id| !id.trim().is_empty())
            && self.correlation_id == other.correlation_id
    }
}
