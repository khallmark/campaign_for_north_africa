-- Private reference catalog, not scenario state or a public browser bundle.
PRAGMA user_version = 1;
PRAGMA application_id = 1129202001;

CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
CREATE TABLE source_file (
    path TEXT PRIMARY KEY, sha256 TEXT NOT NULL CHECK(length(sha256) = 64),
    size INTEGER NOT NULL CHECK(size >= 0)
) STRICT;
CREATE TABLE sql_patch (
    name TEXT PRIMARY KEY, sha256 TEXT NOT NULL CHECK(length(sha256) = 64)
) STRICT;

-- These tables belong to the VASSAL importer. GPIDs never imply canonical identity.
CREATE TABLE vassal_counter (
    gpid TEXT PRIMARY KEY, position INTEGER NOT NULL UNIQUE,
    label TEXT NOT NULL, kind TEXT NOT NULL, side TEXT, nationality_prefix TEXT,
    palette TEXT NOT NULL, source_line INTEGER NOT NULL, printed_name TEXT NOT NULL,
    basic_image TEXT NOT NULL, palette_path_json TEXT NOT NULL CHECK(json_valid(palette_path_json)),
    prototype_refs_json TEXT NOT NULL CHECK(json_valid(prototype_refs_json))
) STRICT;
CREATE TABLE token_asset (
    id TEXT PRIMARY KEY, source_path TEXT NOT NULL, present INTEGER NOT NULL CHECK(present IN (0,1)),
    sha256 TEXT, size INTEGER, public_path TEXT CHECK(public_path IS NULL), private_review_path TEXT,
    CHECK((present = 0 AND sha256 IS NULL AND size IS NULL AND private_review_path IS NULL)
       OR (present = 1 AND sha256 IS NOT NULL AND length(sha256) = 64
           AND size IS NOT NULL AND size >= 0 AND private_review_path IS NOT NULL))
) STRICT;
CREATE TABLE counter_image (
    gpid TEXT NOT NULL REFERENCES vassal_counter(gpid) DEFERRABLE INITIALLY DEFERRED,
    position INTEGER NOT NULL, image_id TEXT NOT NULL REFERENCES token_asset(id) DEFERRABLE INITIALLY DEFERRED,
    origin TEXT NOT NULL, trait_kind TEXT NOT NULL, PRIMARY KEY(gpid, position)
) STRICT;
CREATE TABLE counter_layer (
    gpid TEXT NOT NULL REFERENCES vassal_counter(gpid) DEFERRABLE INITIALLY DEFERRED,
    position INTEGER NOT NULL, origin TEXT NOT NULL, name TEXT NOT NULL, description TEXT NOT NULL,
    initial_state TEXT NOT NULL, first_level_value TEXT NOT NULL, PRIMARY KEY(gpid, position)
) STRICT;
CREATE TABLE counter_level (
    gpid TEXT NOT NULL, layer_position INTEGER NOT NULL, position INTEGER NOT NULL,
    source_index INTEGER NOT NULL, image TEXT NOT NULL, label TEXT NOT NULL,
    PRIMARY KEY(gpid, layer_position, position),
    FOREIGN KEY(gpid, layer_position) REFERENCES counter_layer(gpid, position) DEFERRABLE INITIALLY DEFERRED
) STRICT;
CREATE TABLE palette_candidate (
    gpid TEXT NOT NULL REFERENCES vassal_counter(gpid) DEFERRABLE INITIALLY DEFERRED,
    candidate_gpid TEXT NOT NULL REFERENCES vassal_counter(gpid) DEFERRABLE INITIALLY DEFERRED,
    PRIMARY KEY(gpid, candidate_gpid)
) STRICT;

-- Reviewed catalog state survives VASSAL reimports. Unknown is NULL, never zero.
CREATE TABLE source_citation (
    id TEXT PRIMARY KEY, title TEXT NOT NULL, path TEXT NOT NULL,
    details_json TEXT NOT NULL CHECK(json_valid(details_json))
) STRICT;
CREATE TABLE unit_type (
    id TEXT PRIMARY KEY, position INTEGER NOT NULL UNIQUE, name TEXT NOT NULL,
    details_json TEXT NOT NULL CHECK(json_valid(details_json))
) STRICT;
CREATE TABLE stat_definition (
    id TEXT PRIMARY KEY, position INTEGER NOT NULL UNIQUE, name TEXT NOT NULL,
    unit TEXT NOT NULL, description TEXT NOT NULL, source_refs_json TEXT NOT NULL CHECK(json_valid(source_refs_json))
) STRICT;
CREATE TABLE catalog_record (
    id TEXT PRIMARY KEY, collection TEXT NOT NULL CHECK(collection IN ('entries','profiles')),
    position INTEGER NOT NULL, name TEXT NOT NULL, kind TEXT NOT NULL, side TEXT NOT NULL,
    nationality TEXT NOT NULL, unit_type_id TEXT NOT NULL REFERENCES unit_type(id),
    profile_id TEXT REFERENCES catalog_record(id) DEFERRABLE INITIALLY DEFERRED,
    verification TEXT NOT NULL, note TEXT NOT NULL, details_json TEXT NOT NULL CHECK(json_valid(details_json)),
    UNIQUE(collection, position)
) STRICT;
CREATE TABLE record_source (
    record_id TEXT NOT NULL REFERENCES catalog_record(id), position INTEGER NOT NULL,
    source_id TEXT NOT NULL REFERENCES source_citation(id), PRIMARY KEY(record_id, position)
) STRICT;
CREATE TABLE stat_observation (
    record_id TEXT NOT NULL REFERENCES catalog_record(id), stat_id TEXT NOT NULL REFERENCES stat_definition(id),
    value REAL, status TEXT NOT NULL, raw TEXT, conditional INTEGER NOT NULL CHECK(conditional IN (0,1)),
    note TEXT NOT NULL, source_refs_json TEXT NOT NULL CHECK(json_valid(source_refs_json)),
    PRIMARY KEY(record_id, stat_id),
    CHECK(status IN ('sourced','unverified','provisional','unresolved','not_applicable','conflict')),
    CHECK(status NOT IN ('unresolved','not_applicable','conflict') OR value IS NULL),
    CHECK(status != 'sourced' OR value IS NOT NULL)
) STRICT;
CREATE TABLE correction_evidence (
    id TEXT PRIMARY KEY, source_path TEXT NOT NULL REFERENCES source_file(path),
    locator TEXT NOT NULL, quote TEXT NOT NULL, note TEXT NOT NULL
) STRICT;
CREATE TABLE counter_correction (
    gpid TEXT PRIMARY KEY REFERENCES vassal_counter(gpid) DEFERRABLE INITIALLY DEFERRED,
    label TEXT NOT NULL, evidence_id TEXT NOT NULL REFERENCES correction_evidence(id)
) STRICT;
CREATE VIEW effective_counter AS
SELECT c.*, COALESCE(o.label, c.label) AS effective_label, o.evidence_id
FROM vassal_counter c LEFT JOIN counter_correction o USING(gpid);
CREATE TABLE stat_correction (
    record_id TEXT NOT NULL, stat_id TEXT NOT NULL, value REAL,
    status TEXT NOT NULL CHECK(status IN ('sourced','not_applicable')),
    raw TEXT NOT NULL, conditional INTEGER NOT NULL CHECK(conditional IN (0,1)),
    evidence_id TEXT NOT NULL REFERENCES correction_evidence(id),
    PRIMARY KEY(record_id, stat_id),
    FOREIGN KEY(record_id, stat_id) REFERENCES stat_observation(record_id, stat_id),
    CHECK((status = 'sourced' AND value IS NOT NULL) OR (status = 'not_applicable' AND value IS NULL))
) STRICT;
CREATE VIEW effective_stat AS
SELECT s.record_id, s.stat_id,
    CASE WHEN c.evidence_id IS NULL THEN s.value ELSE c.value END AS value,
    CASE WHEN c.evidence_id IS NULL THEN s.status ELSE c.status END AS status,
    CASE WHEN c.evidence_id IS NULL THEN s.raw ELSE c.raw END AS raw,
    CASE WHEN c.evidence_id IS NULL THEN s.conditional ELSE c.conditional END AS conditional,
    CASE WHEN c.evidence_id IS NULL THEN s.note ELSE e.note END AS note,
    s.source_refs_json AS original_source_refs_json, c.evidence_id,
    s.value AS original_value, s.status AS original_status, s.raw AS original_raw
FROM stat_observation s LEFT JOIN stat_correction c USING(record_id, stat_id)
LEFT JOIN correction_evidence e ON e.id = c.evidence_id;

CREATE TABLE canonical_unit (
    id TEXT PRIMARY KEY, position INTEGER NOT NULL UNIQUE, name TEXT NOT NULL, side TEXT NOT NULL,
    nationality TEXT NOT NULL, role_id TEXT, entity_kind TEXT NOT NULL, identity_status TEXT NOT NULL,
    formation_context TEXT, details_json TEXT NOT NULL CHECK(json_valid(details_json))
) STRICT;
CREATE INDEX unit_name ON canonical_unit(name COLLATE NOCASE);
CREATE TABLE identity_redirect (
    retired_id TEXT PRIMARY KEY, target_id TEXT NOT NULL REFERENCES canonical_unit(id),
    CHECK(retired_id != target_id)
) STRICT;
CREATE TABLE unit_record (
    unit_id TEXT NOT NULL REFERENCES canonical_unit(id), position INTEGER NOT NULL,
    record_id TEXT NOT NULL REFERENCES catalog_record(id), PRIMARY KEY(unit_id, position)
) STRICT;
CREATE INDEX unit_record_record ON unit_record(record_id);
CREATE TABLE unit_source (
    unit_id TEXT NOT NULL REFERENCES canonical_unit(id), position INTEGER NOT NULL,
    source_id TEXT NOT NULL REFERENCES source_citation(id), PRIMARY KEY(unit_id, position)
) STRICT;
CREATE TABLE generated_art (
    unit_id TEXT PRIMARY KEY REFERENCES canonical_unit(id), status TEXT NOT NULL CHECK(status IN ('ready','missing','candidate')),
    portrait_path TEXT, atlas_path TEXT, details_json TEXT NOT NULL CHECK(json_valid(details_json)),
    CHECK(status != 'ready' OR (portrait_path IS NOT NULL AND atlas_path IS NOT NULL
        AND portrait_path LIKE '/sprites/units/%' AND atlas_path LIKE '/sprites/units/%')),
    CHECK(status != 'missing' OR (portrait_path IS NULL AND atlas_path IS NULL))
) STRICT;
CREATE TABLE unit_context (
    unit_id TEXT NOT NULL REFERENCES canonical_unit(id), kind TEXT NOT NULL CHECK(kind IN ('organization_at_arrival','description')),
    details_json TEXT NOT NULL CHECK(json_valid(details_json)), PRIMARY KEY(unit_id, kind)
) STRICT;
CREATE TABLE unit_counter (
    gpid TEXT PRIMARY KEY REFERENCES vassal_counter(gpid) DEFERRABLE INITIALLY DEFERRED,
    unit_id TEXT NOT NULL REFERENCES canonical_unit(id), status TEXT NOT NULL,
    evidence_json TEXT NOT NULL CHECK(json_valid(evidence_json))
) STRICT;
CREATE INDEX unit_counter_unit ON unit_counter(unit_id);
CREATE TABLE cpa_check (
    record_id TEXT PRIMARY KEY REFERENCES catalog_record(id), result TEXT NOT NULL,
    details_json TEXT NOT NULL CHECK(json_valid(details_json))
) STRICT;
CREATE TABLE source_document (
    source_path TEXT PRIMARY KEY REFERENCES source_file(path), kind TEXT NOT NULL,
    titles_json TEXT NOT NULL CHECK(json_valid(titles_json)), tool TEXT NOT NULL,
    status TEXT NOT NULL, text TEXT NOT NULL, note TEXT NOT NULL
) STRICT;

CREATE VIEW unit_token AS
SELECT u.unit_id, u.gpid, u.status AS link_status, c.label, c.basic_image,
       a.present, a.source_path, a.sha256, a.private_review_path, a.public_path
FROM unit_counter u JOIN vassal_counter c USING(gpid)
LEFT JOIN token_asset a ON a.id = c.basic_image;

CREATE VIEW coverage AS SELECT
    0 AS complete,
    (SELECT count(*) FROM canonical_unit) AS canonical_candidates,
    (SELECT count(DISTINCT unit_id) FROM unit_counter) AS linked_canonical_candidates,
    (SELECT count(*) FROM canonical_unit WHERE id NOT IN (SELECT unit_id FROM unit_counter)) AS unresolved_canonical_candidates,
    (SELECT count(*) FROM vassal_counter) AS vassal_slots,
    (SELECT count(*) FROM unit_counter) AS linked_vassal_slots,
    (SELECT count(*) FROM vassal_counter WHERE gpid NOT IN (SELECT gpid FROM unit_counter)) AS unresolved_vassal_slots,
    (SELECT count(*) FROM token_asset WHERE present = 0) AS missing_source_images,
    (SELECT count(*) FROM generated_art WHERE status = 'ready') AS generated_art_ready;
