# Simulation command boundary v1

Status: wire contract only (CNA-160). The sandbox is unchanged. The official
reducer, state, rules kernels and replay verifier are later deliverables; this
document states their obligations, not an assertion that they are implemented.

## Envelopes and versioning

Rust types live in `cna_sim::contract::command`. JSON is UTF-8. `schema_version`
is the integer major version `1`, not a string or floating point number. Unknown
major versions fail deserialization, before any reducer state is accessed.

`Command<P>` has required fields in this serialization order:

| Field | Contract |
| --- | --- |
| `schema_version` | Integer 1 |
| `ruleset_id` | Engine-selected ruleset identifier |
| `ruleset_digest` | `sha256:` followed by 64 lowercase hexadecimal digits, identifying the exact ruleset content |
| `actor` | Stable player/side identifier; not proof of authorization |
| `game_revision` | Unsigned 64-bit expected authoritative revision |
| `command_id` | Nonempty unique ID within the game, reused unchanged on retries |
| `payload` | Typed command payload `P`, defined by the owning kernel |

The reducer validates nonempty identifiers, digest syntax and equality to the
loaded ruleset, actor authority, expected revision, and payload legality. Serde
alone does not validate those semantic constraints. No credentials, timestamps,
host-dependent IDs or raw VASSAL property bags belong in these envelopes.

`StateEnvelope<S>` contains `schema_version`, `ruleset_id`, `ruleset_digest`,
`game_revision`, `payload` in that order. `S` is authoritative state, including
the future engine's RNG state. It is not a viewer projection or export.

`Outcome<E>` is tagged by `status`:

- `accepted`: ordered `events` array, then `new_revision`.
- `rejected`: `redacted_reason` only. No events, revision, state or random draws.

Each `Event<E>` contains `sequence`, `command_id`, typed `payload`, and
`random_draws` (an ordered array, empty when no randomness was consumed).
Outcomes/events inherit the triggering command's schema and ruleset; they do
not independently negotiate a version. Standalone replay packaging is CNA-162.

All envelopes, event records and draw records reject unknown fields. Payloads
must also reject unknown semantic fields. The only forward-compatible optional
namespace demonstrated in the fixtures is `payload.extensions`: an optional
`BTreeMap<String, String>` of non-semantic annotations. Unknown keys in that map
are retained, sorted lexicographically, and may never affect legality, state,
event order or RNG. Future payload types must explicitly opt into that shape;
the generic envelope does not add it automatically. Introducing a required
field or new meaning is a schema/ruleset change, not an ignored annotation.

## Atomic acceptance and rejection

Validate against immutable state first; calculate tentative changes and RNG
draws against a private working copy. Commit state, RNG cursor, events, command
receipt and revision together only on acceptance. Any rejection leaves all of
them unchanged. Accepted commands advance revision by exactly one, even with
zero events; overflow is rejected before work begins. Rejection wording must
not reveal hidden units, resources, positions, legal targets, or RNG state.
Use a stable public reason such as `command unavailable` when distinguishing
causes would disclose information. An arbitrary string is not automatically
redacted; the future projection/reducer owns that policy.

Retries of an already accepted `command_id` with the same complete command
return its recorded outcome without executing twice. Reusing an ID with changed
content is rejected. A rejected command ID is not persisted in authoritative
state. Validation failures consume no RNG; they cannot be used to probe rolls.

## Deterministic event order and random draws

- Event `sequence` is a zero-based global contiguous index, incremented once per
  committed event. Array order is execution order. All events reference the
  accepted command's ID; they are never sorted after random draws are assigned.
- Iterate maps via `BTreeMap` or sorted stable IDs, never `HashMap` iteration,
  DOM order, locale collation, wall time or parallel task completion order.
- An explicit rule priority wins first. Otherwise, ties use ascending UTF-8
  stable IDs; spatial ties use the map contract's canonical hex identifier.
  Explicit player choices remain explicit; deterministic ordering must not
  silently replace a choice required by a rule. Route-specific ties are CNA-163.
- The official PRNG contract is ChaCha8 with a 32-byte seed, stream 0, initial
  word position 0, consuming consecutive `next_u32` words. The reducer leaf must
  pin the implementation/version and publish golden vectors before acceptance.
  No OS RNG, `usize`-dependent sampling, floating RNG or implicit reseeding.
- A `RandomDraw` stores `word_index` (zero-based global word position) and raw
  `value` (u32). Record every consumed word, including rejection-sampling words,
  on the event that consumes it. Use unbiased integer rejection sampling for
  bounded rolls; the kernel documents its range and transformation. A rejection
  rolls back the tentative words, so later accepted commands see the same stream.
- Replay verifies each raw word and its position as well as events and revision;
  it does not trust saved roll values. Overflow of a word/event index is rejected.

## Serialization and verification

Canonical fixture bytes are compact `serde_json::to_string` output: declared
struct field order, enum tag first, sorted map keys, no trailing newline.
Byte-identical round trip applies to these canonical fixtures, not arbitrary
input whitespace/key order. Future typed payloads must avoid unordered maps and
non-finite floats. The fixtures are synthetic, not supported game commands or
certified historical rule behavior.

`cargo test -p cna-sim contract::command` checks command/state/accepted/rejected
fixtures, ordered event and draw preservation, unsupported major versions,
strict unknown-field boundaries, and the explicitly extensible annotation map.
State/RNG rollback, idempotency, authorization and redaction require reducer
integration tests in CNA-261; this leaf cannot prove them without that engine.
