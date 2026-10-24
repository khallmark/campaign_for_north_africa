# Save, replay and viewer export v1

Status: wire contract and synthetic fixtures only (CNA-162). File persistence,
digest validation, migration dispatch and replay verification are CNA-270/271.
No official save loader or automatic redaction is implemented by this contract.

## Private LocalSave

`cna_sim::contract::save::LocalSave<C,S,M>` has these required JSON fields:

| Field | Meaning |
| --- | --- |
| `schema` | Integer major version 1; the same strict version encoding as command envelopes |
| `ruleset_digest`, `map_digest`, `catalog_digest` | Exact loaded reference digests, each `sha256:` plus 64 lowercase hex digits |
| `mode` | Typed mode `M` from official game state; never a free-form rules override |
| `seed` | 32 unsigned bytes, the private ChaCha8 seed from command contract v1 |
| `command_log` | Complete ordered accepted `Command<C>` history from initial revision 0 |
| `snapshot` | `StateEnvelope<S>` after exactly the logged commands, including the RNG cursor and authoritative state |

This is private local authority, not a network export. Keep it out of screenshots,
artifacts, issue comments, analytics and opponent downloads. Credentials and raw
VASSAL property bags are forbidden even here. Importers must strip credentials
in memory before constructing typed domain values, never save a raw intermediate.
All payload types must reject unknown fields; `serde_json::Value` and arbitrary
property maps are not acceptable domain payloads. The generic envelope cannot
enforce this obligation on a caller's choice of Rust type.

Loading must check schema and all three digests against the selected local
reference bundle before installing state or consuming randomness. Validate
nested command/state ruleset references, contiguous revisions, unique accepted
command IDs, mode agreement and every unit/reference ID. Reconstruct from the
matching scenario initial state and seed, replay the complete command log, and
compare the resulting state/RNG cursor with the snapshot. A snapshot is an
acceleration hint, not evidence that the command history is valid. Any failure
leaves the existing session unchanged. Viewer-safe error messages must not print
private values. Replay ordering and draw rules are in [command v1](sim-command-v1.md).

## ViewerExport contains projection history, not replay authority

`ViewerExport<P>` contains only `schema: 1` and ordered `history: P[]`. Each `P`
must be the already-redacted projection for the authorized viewer from CNA-161.
It carries whatever public revision/viewer metadata that projection permits.
Never construct it by serializing LocalSave and deleting a few top-level keys.
No authoritative snapshot, command log, seed/RNG state, hidden unit metadata,
credentials or raw property bags may appear at any depth. Nested redaction is a
producer obligation; a generic container cannot establish it. Exports cannot be
resumed as saves or used to replay authoritative simulation.

## Migration and fixture decisions

No pre-v1 official format has shipped, so v1 is both oldest supported and current.
The private teaching prototype is not a legacy official save.

| Input fixture | Decision | Stage |
| --- | --- | --- |
| `oldest_supported()` — v1, initial snapshot, empty command log | Accept unchanged if references match | Identity migration (1 → 1) |
| `current()` — v1, one accepted synthetic command, revision 1 | Accept after digest/state/replay validation | Identity migration (1 → 1) |
| `current()` with `schema: 2` (or 0) | Reject; never guess a migration | Version decoding |
| `current()` with any one reference digest changed | Reject, even if snapshot deserializes | Reference check before replay |

Future version support needs an explicit source→destination migration entry,
fixtures, and preservation of command/RNG semantics. Never silently relabel a
reference digest or fall back to a different ruleset. Reference changes need a
separately reviewed migration, not merely a JSON shape conversion.

Fixtures live in `contract::save::tests`, using fake reference digests and typed
fixture-only payloads; none contains source-save bytes. Canonical serialization
uses declared struct order, compact JSON, and ordered maps within typed payloads,
as in command v1. `cargo test -p cna-sim contract::save` exercises initial/current
round trips, unknown-major rejection, individually mismatched reference fixtures,
and private-field rejection at the export boundary. Mismatch fixtures decode on
purpose: the later loader must reject them semantically. These tests do not
claim that persistence, full redaction or replay validation already exists.
