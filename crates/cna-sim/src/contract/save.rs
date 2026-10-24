//! Private saves and projection-only exports. Loader policy is CNA-270, not this module.
use super::command::{Command, SchemaVersion, StateEnvelope};
use serde::{Deserialize, Serialize};

/// Private authoritative data. Never use this type as an opponent/viewer export.
/// C, S and M must be strict typed command, state and mode payloads, not JSON bags.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalSave<C, S, M> {
    pub schema: SchemaVersion,
    pub ruleset_digest: String,
    pub map_digest: String,
    pub catalog_digest: String,
    pub mode: M,
    pub seed: [u8; 32],
    /// Complete accepted command history from the scenario's initial revision.
    pub command_log: Vec<Command<C>>,
    /// State after exactly this command history, not an independently trusted cache.
    pub snapshot: StateEnvelope<S>,
}

/// P must be the viewer-specific projection type owned by CNA-161.
/// This generic wire shape does not itself implement redaction or authorize viewers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewerExport<P> {
    pub schema: SchemaVersion,
    pub history: Vec<P>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FixtureState {
        revision_marker: u64,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct FixtureOrder {
        kind: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum FixtureMode {
        FixtureOnly,
    }

    type Save = LocalSave<FixtureOrder, FixtureState, FixtureMode>;
    const RULES: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
    const MAP: &str = "sha256:2222222222222222222222222222222222222222222222222222222222222222";
    const CATALOG: &str = "sha256:3333333333333333333333333333333333333333333333333333333333333333";

    // Synthetic examples, not real historical saves or VASSAL data.
    fn oldest_supported() -> Value {
        json!({
            "schema":1, "ruleset_digest":RULES, "map_digest":MAP,
            "catalog_digest":CATALOG, "mode":"fixture_only", "seed":([7;32]),
            "command_log":[], "snapshot":{
                "schema_version":1, "ruleset_id":"fixture-only", "ruleset_digest":RULES,
                "game_revision":0, "payload":{"revision_marker":0}
            }
        })
    }

    fn current() -> Value {
        let mut fixture = oldest_supported();
        fixture["command_log"] = json!([{
            "schema_version":1, "ruleset_id":"fixture-only", "ruleset_digest":RULES,
            "actor":"allied", "game_revision":0, "command_id":"fixture-1",
            "payload":{"kind":"fixture"}
        }]);
        fixture["snapshot"]["game_revision"] = json!(1);
        fixture["snapshot"]["payload"]["revision_marker"] = json!(1);
        fixture
    }

    #[test]
    fn oldest_and_current_save_fixtures_preserve_history() {
        // v1 is both the current and oldest supported major: no imaginary v0 migration.
        for (fixture, count) in [(oldest_supported(), 0), (current(), 1)] {
            let save: Save = serde_json::from_value(fixture.clone()).unwrap();
            assert_eq!(save.command_log.len(), count);
            assert_eq!(save.snapshot.game_revision, count as u64);
            assert_eq!(serde_json::to_value(&save).unwrap(), fixture);
            let bytes = serde_json::to_vec(&save).unwrap();
            let restored: Save = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
        }
    }

    #[test]
    fn unknown_major_and_digest_mismatch_fixtures_have_distinct_failure_stages() {
        let mut unknown = current();
        unknown["schema"] = json!(2);
        assert!(serde_json::from_value::<Save>(unknown).is_err());
        // Digest mismatch is syntactically valid. CNA-270 must reject it against
        // the selected reference bundle BEFORE installing state or replaying.
        for field in ["ruleset_digest", "map_digest", "catalog_digest"] {
            let mut mismatch = current();
            mismatch[field] =
                json!("sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
            let save: Save = serde_json::from_value(mismatch).unwrap();
            assert_ne!(
                (
                    save.ruleset_digest.as_str(),
                    save.map_digest.as_str(),
                    save.catalog_digest.as_str()
                ),
                (RULES, MAP, CATALOG)
            );
        }
    }

    #[test]
    fn viewer_export_shape_cannot_accept_a_private_save() {
        #[derive(Debug, Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Projection {
            viewer: String,
            visible_unit_ids: Vec<String>,
        }

        let fixture =
            json!({"schema":1,"history":[{"viewer":"allied","visible_unit_ids":["fixture-7"]}]});
        let export: ViewerExport<Projection> = serde_json::from_value(fixture.clone()).unwrap();
        assert_eq!(serde_json::to_value(export).unwrap(), fixture);
        for field in [
            "seed",
            "snapshot",
            "command_log",
            "raw_properties",
            "credentials",
        ] {
            let mut invalid = fixture.clone();
            invalid[field] = json!({});
            assert!(serde_json::from_value::<ViewerExport<Projection>>(invalid).is_err());
        }
        assert!(serde_json::from_value::<ViewerExport<Projection>>(current()).is_err());
        let mut invalid_save = current();
        invalid_save["raw_properties"] = json!({});
        assert!(serde_json::from_value::<Save>(invalid_save).is_err());
    }
}
