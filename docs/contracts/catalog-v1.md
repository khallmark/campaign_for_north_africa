# Catalog consumer contract v1

This contract documents existing data and `cna-catalog` types; it does not certify
the historical roster, migrate data, or supply live game state. It implements
CNA-142, after the layered types in CNA-141. A future Graziani fixture must cite
its own evidence; these examples are synthetic, not a deployment or establishment.

## Wire formats and validation

`docs/catalog-v1.schema.json` extends the existing identity-inventory schema and
exports named definitions for the layered Rust types. Its root validates
`public/data/unit-identities.json` unchanged. `$defs/SourceCatalog` references
the existing `public/data/unit-catalog.json` format; `$defs/UnitReference`
references the supplemental `public/data/unit-reference.json` format, including
counter layers and the existing `formation_reference` v1 contract. Referencing
the original schemas keeps their field types, status vocabularies and validation
rules authoritative rather than copying or weakening them.

```sh
uvx check-jsonschema --schemafile docs/catalog-v1.schema.json public/data/unit-identities.json
uvx check-jsonschema --schemafile docs/unit-catalog.schema.json public/data/unit-catalog.json
uvx check-jsonschema --schemafile docs/unit-reference.schema.json public/data/unit-reference.json
.venv/bin/python -m unittest scripts.test_unit_data.UnitDataTests.test_catalog_v1_schema
cargo test -p cna-catalog schema
```

The Python contract check validates all three committed documents and the
examples below, including rejection controls. The Rust check deserializes the
same documented examples into the exported types. Full source reproduction
remains `npm run test:source-data`; schema acceptance is not source verification.

The current `Inventory` and `Catalog` readers consume their existing formats.
The schema's layered definitions are separately addressable with
`catalog-v1.schema.json#/$defs/CanonicalUnit`, for example. They do **not** add a
new mandatory root collection or make the frontend automatically consume layers.
The supplemental reference uses `units`, `counters`, `token_assets`, `records`
and `profiles`; do not substitute the compact Rust records for those larger wire
records without an explicit adapter. See [SQLite provenance](../catalog/sqlite.md).

Source-citation IDs may be qualified (`stat-note:fixture-unit:anti_armor`);
canonical unit and source-record IDs keep their existing unqualified patterns.
Image and rule-section citations need not invent PDF pages or text lines.
`pdf_page` and `lines` are optional locators, retaining their existing types and
bounds when present; title, path, section and note remain required.

## Identity, representation and instance are different

| Exported type | Contract |
| --- | --- |
| `CanonicalUnitId` | Transparent JSON string. Stable identity, not a name, GPID, role, sprite or live counter ID. |
| `CanonicalUnit` | Reference identity with side, nationality, kind, names, aliases, citations and review status. No current strength or supply. |
| `CounterRepresentation` | Printed label and images for a source counter. Nullable/absent `unit_id` preserves an unresolved link; GPID and sheet are source locators. Several representations may refer to one identity. |
| `StateVersion` | One cited counter/symbol axis and level, not another historical unit or generated sprite. Multiple axes may describe a representation. |
| `ScenarioInstanceRef` | Opaque simulation-owned ID only. Reject extra fields, including `stats`, strength and supplies. Two deployed counters may share one canonical identity but never one instance ID. |
| `AssetRef` | Nullable portrait/atlas paths with artwork status. This compact type does not establish redistribution rights or historical accuracy. |
| `ReferenceValue<T>` | Optional observation plus status, citations and note. Missing/null is unknown, never implicit zero. The schema's generic definition leaves `T` open; use `CountReference` for `u32` and `NumberReference` for `f64`. |
| `Toe` | Cited establishment at a named context, not current personnel/equipment. Counts are `ReferenceValue<u32>` (0 through 4,294,967,295). |
| `Equipment` | Reference characteristics keyed by name, each `ReferenceValue<f64>`, not a live vehicle or inventory balance. |

All layered object types reject unknown fields. Rust `Option` fields can be
omitted or null; serialization emits null for `None`. The generic `ReferenceValue`
does not enforce that a particular review status implies a particular number:
consumers must retain the status, citations and note alongside the value.

Status enums are serialized in snake_case:

- `IdentityStatus`: `provisional`, `source_identified`, `alias_resolved`,
  `verified`, `unresolved`, `fixture`. The legacy inventory intentionally permits
  only its existing first three values; this contract does not promote entries.
- `AssetStatus`: `missing`, `candidate`, `ready`. For the legacy inventory,
  `sprite.status = ready` requires both canonical generated-art paths. Runtime
  `Artwork::ready()` also requires both paths; candidate art is not ready art.
- `ValueStatus`: `sourced`, `unverified`, `provisional`, `unresolved`,
  `not_applicable`, `conflict`. Supplemental-reference stat statuses retain their
  own schema vocabulary; do not silently reinterpret them as this enum.

## Examples checked against Rust and JSON Schema

The following single example object contains one example per reusable definition.
Its keys identify definitions for the tests; the object is **not** a new document
envelope. `fixture-*` values deliberately avoid claims about historical units.

```json
{
  "CanonicalUnitId": "fixture-unit",
  "IdentityStatus": "fixture",
  "AssetStatus": "missing",
  "ValueStatus": "unresolved",
  "CanonicalUnit": {
    "id": "fixture-unit", "side": "axis", "nationality": "fixture",
    "kind": "formation", "names": ["Fixture unit"], "aliases": [],
    "source_refs": ["fixture-source"], "status": "fixture"
  },
  "CounterRepresentation": {
    "unit_id": null, "gpid": "fixture-slot", "sheet": null,
    "printed_label": "Unresolved fixture", "images": []
  },
  "StateVersion": {"axis": "cohesion", "level": "printed-1", "source": "fixture-source"},
  "ScenarioInstanceRef": {"id": "fixture-instance-2"},
  "AssetRef": {"portrait": null, "atlas": null, "status": "missing"},
  "ReferenceValue": {"value": "printed dash", "status": "unresolved", "source_refs": ["fixture-source"], "note": "Meaning unresolved"},
  "CountReference": {"value": 0, "status": "sourced", "source_refs": ["fixture-source"], "note": "Explicit zero in synthetic source"},
  "NumberReference": {"value": 2.5, "status": "provisional", "source_refs": ["fixture-source"], "note": "Synthetic characteristic"},
  "Toe": {
    "unit_id": "fixture-unit", "context": "Synthetic establishment, not current strength",
    "personnel": {"value": null, "status": "unresolved", "source_refs": [], "note": "Unknown"},
    "equipment": {"fixture-model": {"value": 3, "status": "provisional", "source_refs": ["fixture-source"], "note": "Synthetic count"}}
  },
  "Equipment": {
    "id": "fixture-model", "name": "Fixture equipment",
    "characteristics": {"rate": {"value": 2.5, "status": "provisional", "source_refs": ["fixture-source"], "note": "Synthetic reference only"}}
  }
}
```

## Redirects and joins

`identity_redirects` maps a retired canonical ID to its replacement, for example
`{"fixture-old": "fixture-middle", "fixture-middle": "fixture-unit"}`. Resolve
the whole chain with `Inventory::find`; never copy a retired entry into a second
live unit. Unknown lookup returns `None`. `Inventory::parse` rejects duplicate
live IDs, duplicate redirect keys, cycles, dangling targets and a live ID also
used as a redirect source. JSON Schema validates the map's shape, **not** graph
integrity or duplicate keys; use the Rust parser as well as schema validation.

Join source records through `source_entry_ids`/`source_row_map` in the inventory,
or `source_record_ids` in the supplemental reference. Resolve counter GPIDs via
explicit reviewed links; names alone are not identity evidence. Loadouts and
counter layers are not additional units by default. Never deduplicate live game
instances by canonical ID, source-row ID or GPID.

## Reference data is not intelligence

Land rules §3.61–3.62 limit opposing counter information. The catalog contains
public reference facts, not permission to reveal an opponent's current strength,
CP, fuel, ammunition, water, hidden state or orders. Scenario state and disclosure
belong to the simulation's viewer projection. The current hot-seat mode is
explicitly open-information and manually adjudicated; this contract does not
claim that a limited-intelligence projection is implemented.

Schema version 1, existing field types and redirect meanings remain unchanged.
A breaking consumer-shape or meaning change requires a new version and migration;
newly reviewed source records do not by themselves certify gameplay or artwork.
