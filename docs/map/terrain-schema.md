# Geography records and source vocabulary

`crates/cna-map/src/terrain.rs` and [map-v1.schema.json](map-v1.schema.json)
define data, not costs. The vocabulary itself assigns no terrain to the real map;
the [regenerable research model](model.md) contains source-labelled transcriptions
with explicit review status, complete source-grid geometry and a private SQLite export.
Validate imported JSON with the schema **and** deserialize it as `Vec<MapRecord>`:
the schema checks metadata and structure; Rust additionally checks canonical side
identity. Record provenance requires source references, reviewer and date; an
adjudication identifier is optional. No type defaults to Clear.

## Exact §8.37 rows and corrections

L21 `Rules_Land_07-2021.pdf` page 15 (§8.37) points to Charts and Tables.
The supplied `images/TEC.png` is captioned **[8.37] TERRAIN EFFECTS CHART**.
The schema records exact row labels (without superscript footnote numbers):

- Terrain/effect rows: Clear, Gravel, Salt Marsh, Heavy Vegetation, Rough,
  Mountain, Delta, Desert, Major City, Swamp, Village/Bir/Oasis, Level One,
  Level Two, Level Three, Friendly Minefield, Enemy Minefield.
- Level One/Two/Three appear under **Fortifications**, a heading, not a fourth
  fortification level. Minefield friendliness is relative to a player; these rows
  are effect vocabulary, not immutable geographical ownership.
- Hexsides: Railroad, Road, Track, Ridge, Up Slope, Down Slope, Up Escarpment,
  Down Escarpment, Wadi, Major River, Minor River. Preserve **Railroad**, the
  chart label, rather than changing the source label to “Railway.”

Both L21 §8.37 and `incorporated errata.txt` correct the original chart:

1. Footnote **(4) belongs to Major City, not Swamp**. The supplied TEC image
   already shows this; Alexandria and Cairo are Level Three, other major cities
   Level Two. This is source evidence, not automatic assignments in this schema.
2. Footnote **(8) governs tracks**: tracks halve terrain feature entry costs and
   hex/hexside breakdown values, with the stated descending-escarpment exception.
   They do **not** cost a fixed 1 CP. Simulation owns that arithmetic.

`HexsideFeature::UpSlope` and `UpEscarpment` are oriented from the canonical
`HexsideId.hex()` toward its neighbour. Reverse crossings use `reversed()`.
Input side JSON must already be canonical; it is rejected rather than silently
flipped (which would reverse an attached directional feature).

## Supplemental feature references are not TEC rows

| Feature | Evidence |
| --- | --- |
| Coast | AL21 §36.3: flying-boat basins are placed in coastal hexes. Domain marker only; no invented movement cost. |
| Sea | L21 §5 Commonwealth Fleet Phase assigns ships to sea/coastal hexes. Domain marker only. |
| Pipeline | AL21 §52.2 Water Pipelines, not §8.37. |
| Major city / village / bir / oasis | TEC row labels; AL21 §52.1 and §52.3 distinguish their water-source roles. |
| Well | AL21 §52.1. |
| Airfield / landing strip / flying-boat basin | AL21 §36.1 / §36.2 / §36.3. |
| Fortification | L21 §25.1, levels One/Two/Three only. |
| Port | AL21 §55.3 and `images/55.3 Port Capacity and Efficiency Level Chart.png`. |

Ports require an `efficiency_ref` naming a chart row: Tripoli, Bizerta, Alexandria,
Tobruk, Benghazi, Mersa Matruh, Bardia, Sollum, Derna, or All others. It is a row
reference, not a cached capacity, damage state, or inferred location.

## Unresolved minor city

The ticket requests “minor city,” but no authoritative category has been
established in the inspected L21/AL21 text or TEC/port charts. This is not proof
that every supplied scan lacks the term. The simulation/source owner confirmed
the same current limitation and directed keeping such observations unresolved.
Use `{"status":"unresolved","source_label":"Minor City ..."}` and an
adjudication reference. **There is no known `minor_city` enum and no cost**.
It cannot be promoted to reviewed/provisional/fixture merely by changing status:
those statuses require a typed known value, while unresolved records forbid one.
Unknown terrain uses the same unresolved representation, never null or Clear.

## Synthetic examples

[terrain-example.json](examples/terrain-example.json) is deliberately hand-written
to exercise every record kind and status. **Every location/value/status in it is
synthetic**, including the example labelled `reviewed`; it does not certify a port
or any terrain at C3617. It lives under docs, not production data. Only the type
and chart vocabulary are grounded in sources. Historical records belong to E4.

```sh
cargo test --locked -p cna-map terrain
uvx check-jsonschema --schemafile docs/map/map-v1.schema.json docs/map/examples/terrain-example.json
```
