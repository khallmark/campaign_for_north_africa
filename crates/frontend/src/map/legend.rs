//! Visual key for the active renderer, not a catalogue of unused art assets.
use super::geography::terrain_color;
use cna_map::terrain::Terrain;
use std::fmt::Write;

pub(super) fn render() -> String {
    let mut html = String::from(
        r#"<details class="geography-legend" name="map-reference">
        <summary>Map legend <span>Active layers</span></summary>
        <div class="legend-content" role="region" aria-label="Map legend" tabindex="0">
        <p class="legend-empty">No map layers selected. Enable a viewer option above to see its key.</p>
        <section class="legend-source-only"><h3>Source scan</h3><p>Printed source imagery is visible. Its original symbols are not the custom overlay symbols below; the printed key has not been transcribed here.</p><p class="legend-art-only">Original terrain is blended over the scan at 55% opacity. Turn it off for an unobscured source comparison.</p></section>
        <section class="legend-art-only"><h3>Terrain tiles</h3><p>High-quality terrain · Original terrain enabled</p>
        <p>Original terrain textures blend where materials are observed. Blends show presence, not surveyed boundaries or full-hex coverage.</p>
        <ul class="legend-samples">
        <li><span class="legend-swatch legend-unknown" aria-hidden="true"></span><span>Unknown / unmapped<small>Neutral paper · Not Clear</small></span></li>"#,
    );
    for (asset, label) in [
        ("clear", "Clear"),
        ("desert", "Desert / sand"),
        ("gravel", "Gravel"),
        ("rough", "Rough"),
        ("mountain", "Mountain"),
        ("delta", "Delta"),
        ("salt_marsh", "Salt marsh"),
        ("swamp", "Swamp"),
        ("heavy_vegetation", "Heavy vegetation"),
    ] {
        write!(html, r#"<li><img class="legend-swatch" src="/art/terrain-v1/surfaces/{asset}.png" alt="" loading="lazy"><span>{label}</span></li>"#).unwrap();
    }
    html.push_str(
        r#"</ul></section><section class="legend-tint-only"><h3>Terrain evidence tints</h3><p>Original terrain is off. Colored hexes show observed terrain, not surveyed extents. Overlapping observations can mix these tints; no tint means unmapped, not Clear.</p><ul class="legend-samples">"#,
    );
    for (terrain, label) in [
        (Terrain::Clear, "Clear"),
        (Terrain::Desert, "Desert / sand"),
        (Terrain::Gravel, "Gravel"),
        (Terrain::Mountain, "Mountain"),
        (Terrain::Delta, "Delta"),
        (Terrain::SaltMarsh, "Salt marsh"),
        (Terrain::Swamp, "Swamp / vegetation"),
        (Terrain::Rough, "Other typed terrain"),
    ] {
        write!(html, r#"<li><svg viewBox="0 0 60 32" aria-hidden="true"><rect class="legend-terrain" fill="{}" x="5" y="3" width="50" height="26"/></svg><span>{label}</span></li>"#, terrain_color(&terrain)).unwrap();
    }
    html.push_str(
        r#"</ul></section><section class="legend-geography-only"><h3>Routes &amp; construction</h3>
        <p>Geography shows schematic crossings, not exact paths or movement permission. Parallel display lanes separate a track and construction sharing a crossing; their spacing is not surveyed.</p><ul class="legend-samples">"#,
    );
    for (class, label, ties) in [
        ("legend-route road", "Road", ""),
        (
            "legend-route railroad",
            "Railroad",
            "legend-route railroad railroad-ties",
        ),
        ("legend-route track", "Track", ""),
        ("legend-route pipeline", "Pipeline", ""),
        ("legend-construction unfinished-road", "Unfinished road", ""),
        (
            "legend-construction unfinished-railroad",
            "Unfinished railroad",
            "legend-construction construction-ties",
        ),
    ] {
        html.push_str(r#"<li><svg viewBox="0 0 60 32" aria-hidden="true">"#);
        if class == "legend-route track" {
            html.push_str(r#"<path class="geography-track-casing" d="M3 16H57"/>"#);
        }
        write!(html, r#"<path class="{class}" d="M3 16H57"/>"#).unwrap();
        if !ties.is_empty() {
            write!(html, r#"<path class="{ties}" d="M3 16H57"/>"#).unwrap();
        }
        write!(html, "</svg><span>{label}</span></li>").unwrap();
    }
    html.push_str(
        r#"</ul><p>Violet construction marks remain unresolved and are excluded from network links. Other routes may also be provisional; inspect their evidence.</p></section>
        <section class="legend-geography-only"><h3>Water &amp; hexsides</h3><ul class="legend-samples">
        <li><svg viewBox="0 0 60 32" aria-hidden="true"><rect class="legend-water" x="5" y="3" width="50" height="26"/></svg><span>Water evidence</span></li>
        <li><svg viewBox="0 0 60 32" aria-hidden="true"><rect class="legend-water mixed_water_land" x="5" y="3" width="50" height="26"/></svg><span>Mixed water / land</span></li>
        <li><svg viewBox="0 0 60 32" aria-hidden="true"><path class="source-water" d="M4 3Q40 4 22 17Q9 28 56 25L56 32Q-8 34 10 15Q24 5 4 10Z"/></svg><span>Source water shape<small>Original channel width · display evidence only</small></span></li>
        <li><svg viewBox="0 0 60 32" aria-hidden="true"><path class="source-wadi" d="M4 3Q40 4 22 17Q9 28 56 25L56 32Q-8 34 10 15Q24 5 4 10Z"/></svg><span>Source wadi shape<small>Dry channel · not flowing water</small></span></li>
        <li><svg viewBox="0 0 60 32" aria-hidden="true"><path class="source-escarpment" d="M3 7L25 17L57 7V16L49 17L48 23L42 20L39 25L32 23L25 28L17 23L13 25L9 18L3 19Z"/></svg><span>Source escarpment face<small>Printed fringe · Up/Down unresolved</small></span></li>
        <li><svg viewBox="0 0 60 32" aria-hidden="true"><path class="legend-route side" d="M3 16H57"/></svg><span>Other natural hexside<small>Ridge, slope, escarpment, coast or sea</small></span></li>
        </ul><p>Source water, dry-wadi and escarpment shapes retain printed widths, bends and scarp faces. Whole-cell water evidence tints appear only with Original terrain off; they are not shorelines. Shapes are automated evidence, not Major/Minor River, sea/coast or Up/Down Escarpment classifications. Source overprint gaps remain; no movement or crossing rules follow from these shapes. Other natural hexsides share one style; use Hex evidence to identify the feature.</p></section>
        <section class="legend-points-only"><h3>Places &amp; facilities</h3><p>Original symbols appear at detail scale with Original terrain and Geography enabled. A symbol gives no capacity or operational status.</p><ul class="legend-samples">"#,
    );
    for (asset, label) in [
        ("major_city", "Major city"),
        ("village", "Village"),
        ("bir", "Bir"),
        ("oasis", "Oasis"),
        ("well", "Well"),
        ("village_bir_oasis", "Village / bir / oasis"),
        ("port", "Port"),
        ("airfield", "Airfield"),
        ("landing_strip", "Landing strip"),
        ("flying_boat_basin", "Flying-boat basin"),
        ("fortification_level_one", "Fortification I"),
        ("fortification_level_two", "Fortification II"),
        ("fortification_level_three", "Fortification III"),
    ] {
        write!(html, r#"<li><img class="legend-point" src="/art/terrain-v1/overlays/{asset}.png" alt="" loading="lazy"><span>{label}</span></li>"#).unwrap();
    }
    html.push_str(
        r#"</ul></section><section class="legend-tint-only"><h3>Locations</h3><ul class="legend-samples"><li><svg viewBox="0 0 60 32" aria-hidden="true"><circle class="legend-location" cx="30" cy="16" r="4"/></svg><span>Location marker<small>Original terrain off</small></span></li></ul></section>
        <section class="legend-geography-only"><h3>Unclassified evidence</h3><ul class="legend-samples">
        <li><svg viewBox="0 0 60 32" aria-hidden="true"><path class="legend-route unresolved" d="M3 16H57"/></svg><span>Unclassified hexside</span></li>
        <li><svg viewBox="0 0 60 32" aria-hidden="true"><circle class="legend-location unresolved" cx="30" cy="16" r="4"/></svg><span>Unclassified location</span></li>
        <li><svg class="source-point unresolved" viewBox="0 0 60 32" aria-hidden="true"><circle class="source-point-symbol" cx="30" cy="16" r="4"/></svg><span>Measured Village/Bir dot<small>Category unresolved</small></span></li>
        <li><svg class="source-point" viewBox="0 0 60 32" aria-hidden="true"><g transform="translate(30 18)"><ellipse class="source-point-symbol" rx="7" ry="4"/><path class="source-point-symbol" d="M0 0V-10M-5-6Q0-12 5-6"/></g></svg><span>Measured oasis glyph</span></li>
        <li><svg class="source-point" viewBox="0 0 60 32" aria-hidden="true"><g transform="translate(30 18)"><path class="source-point-symbol" d="M0-7V7M-5-3H5M-7 1Q-7 7 0 7Q7 7 7 1"/><circle class="source-point-symbol" cy="-9" r="2"/></g></svg><span>Measured port glyph</span></li>
        </ul><p>Measured glyphs and detail-scale labels follow source positions with Geography enabled, independently of Original terrain. Labels associate names with glyphs, not operational categories or capacities.</p><p>Magenta marks are unresolved, not missing roads or railroads. Transcription is incomplete; no mark does not mean no feature.</p></section>
        <section class="legend-grid-only"><h3>Hex grid</h3><ul class="legend-samples"><li><svg viewBox="0 0 60 32" aria-hidden="true"><path class="legend-grid" d="M16 3H44L52 16L44 29H16L8 16Z"/></svg><span>Source-grid hex<small>Labels at detail scale</small></span></li></ul><p>Section seams and official playable identities remain unresolved.</p></section>
        <section class="legend-inspect-only"><h3>Inspect mode</h3><p>Click a hex or press Enter on the map to inspect the viewport center. Hex evidence opens with source citations.</p></section>
        <section class="legend-selection-only"><h3>Inspected hex</h3><ul class="legend-samples"><li><svg viewBox="0 0 60 32" aria-hidden="true"><path class="legend-selected" d="M16 3H44L52 16L44 29H16L8 16Z"/></svg><span>Selected outline</span></li></ul><p>The outline stays visible after Inspect mode is turned off. Inspect a hex for <b>reviewed</b>, <b>provisional</b> or <b>unresolved</b> records. These are evidence states, not game-rule costs.</p></section>
        <p class="legend-layer-note">Only enabled layers are keyed here. Use Hex identity to look up source evidence in any viewing mode.</p>
        </div></details>"#,
    );
    html
}
