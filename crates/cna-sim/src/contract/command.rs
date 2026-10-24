//! Command boundary v1. See docs/contracts/sim-command-v1.md for reducer obligations.
use serde::{Deserialize, Serialize};

/// An integer major version; unsupported versions cannot deserialize as v1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub enum SchemaVersion {
    V1,
}

impl TryFrom<u32> for SchemaVersion {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::V1),
            _ => Err("unsupported simulation envelope major version; expected 1"),
        }
    }
}

impl From<SchemaVersion> for u32 {
    fn from(_: SchemaVersion) -> Self {
        1
    }
}

/// Payloads are typed by their owning rules kernel, not accepted as unchecked JSON.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command<P> {
    pub schema_version: SchemaVersion,
    pub ruleset_id: String,
    pub ruleset_digest: String,
    pub actor: String,
    pub game_revision: u64,
    pub command_id: String,
    pub payload: P,
}

/// The revision and ruleset identify the authoritative state used for validation.
/// This is not a viewer export and must never be sent directly to an opponent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateEnvelope<S> {
    pub schema_version: SchemaVersion,
    pub ruleset_id: String,
    pub ruleset_digest: String,
    pub game_revision: u64,
    pub payload: S,
}

/// Raw ChaCha8 u32 words, including words discarded by unbiased range sampling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RandomDraw {
    pub word_index: u64,
    pub value: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event<E> {
    /// Zero-based global event position, not a wall-clock timestamp.
    pub sequence: u64,
    pub command_id: String,
    pub payload: E,
    pub random_draws: Vec<RandomDraw>,
}

/// Rejections carry no events, revision, private state, or RNG draws.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome<E> {
    Accepted {
        events: Vec<Event<E>>,
        new_revision: u64,
    },
    Rejected {
        redacted_reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;
    use std::collections::BTreeMap;

    // Wire fixtures are synthetic contract examples, not implemented game orders.
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FixturePayload {
        kind: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        extensions: BTreeMap<String, String>,
    }

    const COMMAND: &str = r#"{"schema_version":1,"ruleset_id":"fixture-only","ruleset_digest":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","actor":"allied","game_revision":7,"command_id":"fixture-23","payload":{"kind":"fixture"}}"#;
    const EXTENDED_COMMAND: &str = r#"{"schema_version":1,"ruleset_id":"fixture-only","ruleset_digest":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","actor":"axis","game_revision":8,"command_id":"fixture-24","payload":{"kind":"fixture","extensions":{"a-note":"retained","z-note":"also retained"}}}"#;
    const ACCEPTED: &str = r#"{"status":"accepted","events":[{"sequence":18,"command_id":"fixture-23","payload":{"kind":"fixture-a"},"random_draws":[{"word_index":31,"value":4294967295},{"word_index":32,"value":17}]},{"sequence":19,"command_id":"fixture-23","payload":{"kind":"fixture-b"},"random_draws":[]}],"new_revision":8}"#;
    const REJECTED: &str = r#"{"status":"rejected","redacted_reason":"command unavailable"}"#;
    const STATE: &str = r#"{"schema_version":1,"ruleset_id":"fixture-only","ruleset_digest":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","game_revision":7,"payload":{"kind":"fixture-state"}}"#;

    fn round_trip<T: DeserializeOwned + Serialize>(fixture: &str) {
        let value: T = serde_json::from_str(fixture).unwrap();
        assert_eq!(serde_json::to_string(&value).unwrap(), fixture);
    }

    #[test]
    fn command_and_outcome_fixtures_round_trip_byte_identically() {
        for fixture in [COMMAND, EXTENDED_COMMAND] {
            round_trip::<Command<FixturePayload>>(fixture);
        }
        for fixture in [ACCEPTED, REJECTED] {
            round_trip::<Outcome<FixturePayload>>(fixture);
        }
        round_trip::<StateEnvelope<FixturePayload>>(STATE);
    }

    #[test]
    fn command_versions_and_extension_boundaries_are_enforced() {
        for version in ["0", "2", "4294967296", "-1", "1.0", "1.5", "\"1\"", "null"] {
            let replace = |s: &str| {
                s.replace(
                    "\"schema_version\":1",
                    &format!("\"schema_version\":{version}"),
                )
            };
            assert!(serde_json::from_str::<Command<FixturePayload>>(&replace(COMMAND)).is_err());
            assert!(
                serde_json::from_str::<StateEnvelope<FixturePayload>>(&replace(STATE)).is_err()
            );
        }
        let unknown_envelope = COMMAND.replacen('{', "{\"optional_future_field\":true,", 1);
        assert!(serde_json::from_str::<Command<FixturePayload>>(&unknown_envelope).is_err());
        let unknown_payload = COMMAND.replace(
            "\"kind\":\"fixture\"",
            "\"kind\":\"fixture\",\"optional_future_field\":true",
        );
        assert!(serde_json::from_str::<Command<FixturePayload>>(&unknown_payload).is_err());
        let extended: Command<FixturePayload> = serde_json::from_str(EXTENDED_COMMAND).unwrap();
        assert_eq!(extended.payload.extensions["z-note"], "also retained");
        // Input key order cannot determine event processing or serialized extension order.
        let unsorted = EXTENDED_COMMAND.replace(
            "\"a-note\":\"retained\",\"z-note\":\"also retained\"",
            "\"z-note\":\"also retained\",\"a-note\":\"retained\"",
        );
        let parsed: Command<FixturePayload> = serde_json::from_str(&unsorted).unwrap();
        assert_eq!(serde_json::to_string(&parsed).unwrap(), EXTENDED_COMMAND);
    }

    #[test]
    fn rejection_shape_cannot_hide_events_or_private_state() {
        for field in [
            "\"events\":[]",
            "\"new_revision\":8",
            "\"random_draws\":[]",
            "\"state\":{}",
        ] {
            let invalid = REJECTED.replacen('{', &format!("{{{field},"), 1);
            assert!(serde_json::from_str::<Outcome<FixturePayload>>(&invalid).is_err());
        }
        let unknown_event =
            ACCEPTED.replace("\"sequence\":18", "\"sequence\":18,\"private_state\":{}");
        assert!(serde_json::from_str::<Outcome<FixturePayload>>(&unknown_event).is_err());
    }
}
