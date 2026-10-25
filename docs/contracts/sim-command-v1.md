# Simulation command boundary v1

Status: command wire contract (CNA-160), with state representation and calendar
arithmetic in `cna_sim::official::state` (CNA-260), and transactional execution in
`cna_sim::official::reducer` (CNA-261). Only a no-op kernel ships; official game
rules and the save/replay verifier remain later deliverables. The sandbox is
unchanged. Protocol tests are not assertions of implemented game rules.

## Authoritative state representation

`GameState` stores stable instance IDs separately from catalog unit IDs, current
TOE points by weapon-system reference, exact dimensional quantities for fuel,
ammunition, stores, water and trucks, counter-state references, assignment and
attachment, cargo batches, commitments, reference digests and the explicit RNG
seed/cursor. Maps and sets have sorted iteration. No HP or percentage resource
model is used. Unit supplies and separate cargo-batch contents are distinct
stocks, not mirrored copies; later transfer rules must conserve their total.

Deserialize, then call `GameState::validate` with the loaded catalog's ID set
before installing state. This rejects dangling organization, cargo and commitment
references, unknown catalog IDs, blank identifiers and malformed digest syntax.
It does not certify equipment values, organizational legality, allegiance,
occupancy, map membership, historical placement, or the contents behind digests.
These require the relevant rules and loader checks. Diagnostics contain internal
references and must not be returned as viewer-facing command errors.

Calendar arithmetic follows L21 §5.1 (physical p12): three Operations Stages per
weekly Game-Turn. The supplied printed track labels GT1 September III 1940 and
GT111 January I 1943, with four labelled weeks per month, not Gregorian dates.
Advancing from stage 3 reaches stage 1 of the next Game-Turn; the final stage of
GT111 has no successor. This helper does not execute a phase or authorize an End
Stage command. The future phase engine owns those transitions and scenario limits.

Stored mode names identify the planned four profiles, not implemented kernels or
an approved mode matrix. RNG fields preserve the command contract's ChaCha8
stream-0 seed and word cursor; `official::rng` now supplies recorded draws.
Pending ADJ-SHIP and ADJ-MODES decisions are not resolved by storing these fields.

`cargo test --locked -p cna-sim --test official_state` checks deterministic state
round trips, exact fractional quantities, dangling references and all 333 calendar
positions. `cargo test --locked -p cna-map --test hex` covers the additive string
serialization of `MapLocation` through its existing strict parser, including
off-map boxes; no aliases or adjacency semantics change.

## Transactional execution (CNA-261)

`Session<P, E>` owns state, ordered events, the next event index and accepted
command receipts. `Session::new` validates the initial state's references and
takes an authorized actor-to-side map plus the selected catalog ID set. It
starts a new event history; it is **not** a save loader. Reconstructing a running
session from `GameState` alone loses receipts and would break retry safety.
The session's serialization is an internal diagnostic snapshot, not a public
projection or versioned save format; deserialization is intentionally absent.

`Session::reduce(authenticated_actor, command, kernel)` checks the supplied
principal against the session's actor map and command before looking up a receipt.
The host must supply the authenticated principal independently of the request,
and bind the trusted Rust kernel to the loaded ruleset. This API does not perform
login, accept callbacks from clients, or make authoritative events safe to expose
to a viewer. Kernel payload equality must include every field, including optional
annotations, for complete-command retry comparison.

The kernel receives a `Transaction<E>` containing a copied game state and emits
ordered events through `tx.emit`. Its event callback receives a `RecordedRng`;
every consumed word attaches to that event. A failed event poisons the transaction
even if its error is ignored. State, cursor, events, receipt and revision commit
together, only after the kernel succeeds and reference validation passes. Direct
kernel edits to revision, reference digests or the stored RNG reject. The kernel
must not perform I/O or mutate anything outside the transaction.

Only `Noop {}` and the `noop` kernel are provided. They acknowledge a command with
one empty event and advance revision, but do not change calendar, resources or RNG.
No browser consumer is migrated to this API. Synthetic test kernels exercise
random draws and rejection after tentative mutations; they are not game orders.

The implementation pins `rand_chacha = 0.3.1` with default features disabled
(no OS randomness). `RecordedRng::bounded(upper)` returns `0..upper`, discarding
words below `(2^32 mod upper)` and recording the discarded words too. The retained
range has a count divisible by `upper`, so modulo introduces no bias. Even an
upper bound of one consumes a word. Game kernels define their own die offsets.
The stored u64 word cursor rejects exhaustion rather than wrapping the library's
wider stream position.

Golden vectors use independently published [Strombergson draft -01 §3 TC1/TC2](https://www.ietf.org/archive/id/draft-strombergson-chacha-test-vectors-01.txt),
256-bit keys, 8 rounds, zero IV: two full blocks for the zero key and nonzero-key
words, interpreted little-endian. The source is an expired informational draft,
not an IETF standard. Tests also resume at word positions across block boundaries.

`cargo test --locked -p cna-sim official::reducer` and
`cargo test --locked -p cna-sim --test official_traces` execute the same public
integration suite. Its bounded property test exhausts 256 seeds/private stocks
and ten rejection causes, checking byte-identical session rollback and a single
public rejection shape. Named command traces compare events against explicit
golden words and event digests across runs, including retries, rejected draws,
empty-event acceptance and non-semantic annotations. They do not yet verify
untrusted saved replay streams; that remains the save/replay leaf.

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
  word position 0, consuming consecutive `next_u32` words. CNA-261 pins the
  implementation/version and verifies independent golden vectors as above.
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
State/RNG rollback, idempotency, actor gating and uniform rejection are tested by
the CNA-261 reducer integration suite. Login and viewer projections are separate
boundaries; these tests do not certify them.
