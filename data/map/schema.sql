-- Derived research database. No default terrain or official movement authority.
PRAGMA foreign_keys = ON;
CREATE TABLE metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
CREATE TABLE sources (path TEXT PRIMARY KEY, sha256 TEXT NOT NULL CHECK(length(sha256)=64)) STRICT;
CREATE TABLE hexes (
  id TEXT PRIMARY KEY, section TEXT NOT NULL, col INTEGER NOT NULL, row INTEGER NOT NULL,
  center_x REAL NOT NULL, center_y REAL NOT NULL, corners TEXT NOT NULL CHECK(json_array_length(corners)=6),
  terrain_status TEXT NOT NULL DEFAULT 'unmapped' CHECK(terrain_status IN ('unmapped','partial')),
  UNIQUE(section,col,row)
) STRICT;
CREATE TABLE hexsides (
  hex TEXT NOT NULL REFERENCES hexes(id), direction TEXT NOT NULL,
  other TEXT NOT NULL REFERENCES hexes(id), edge TEXT NOT NULL CHECK(json_array_length(edge)=2),
  PRIMARY KEY(hex,direction), UNIQUE(hex,other), CHECK(hex<>other),
  CHECK(direction IN ('east','north_west','north_east'))
) STRICT;
CREATE INDEX hexsides_other ON hexsides(other);
CREATE TABLE cell_measurements (
  hex TEXT PRIMARY KEY REFERENCES hexes(id),
  surface TEXT NOT NULL CHECK(surface IN ('land','water','mixed_water_land','unresolved')),
  evidence TEXT NOT NULL CHECK(json_valid(evidence))
) STRICT;
CREATE TABLE examinations (
  hex TEXT NOT NULL REFERENCES cell_measurements(hex),
  family TEXT NOT NULL CHECK(family IN ('surface','terrain','transport','hexside','locations')),
  status TEXT NOT NULL CHECK(status IN ('detected','candidate','not_detected','unresolved')),
  PRIMARY KEY(hex,family)
) STRICT;
CREATE TABLE side_examinations (
  hex TEXT NOT NULL, direction TEXT NOT NULL,
  evidence TEXT NOT NULL CHECK(json_valid(evidence)),
  PRIMARY KEY(hex,direction), FOREIGN KEY(hex,direction) REFERENCES hexsides(hex,direction)
) STRICT;
CREATE VIEW examination_coverage AS
SELECT h.section, e.family, e.status, count(*) AS hexes
FROM examinations e JOIN hexes h ON h.id=e.hex
GROUP BY h.section,e.family,e.status;
CREATE TABLE surveys (id TEXT PRIMARY KEY, label TEXT NOT NULL, notes TEXT NOT NULL) STRICT;
CREATE TABLE observations (
  id TEXT PRIMARY KEY, survey TEXT NOT NULL REFERENCES surveys(id),
  kind TEXT NOT NULL CHECK(kind IN ('terrain','hexside','point')),
  hex TEXT NOT NULL REFERENCES hexes(id), direction TEXT,
  status TEXT NOT NULL CHECK(status IN ('reviewed','provisional','unresolved')),
  value TEXT, source_label TEXT, provenance TEXT NOT NULL CHECK(json_valid(provenance)),
  depiction TEXT CHECK(depiction IN ('unfinished_road','unfinished_railroad')),
  FOREIGN KEY(hex,direction) REFERENCES hexsides(hex,direction),
  CHECK((kind='hexside' AND direction IS NOT NULL) OR (kind<>'hexside' AND direction IS NULL)),
  CHECK(depiction IS NULL OR (kind='hexside' AND status='unresolved')),
  CHECK((status='unresolved' AND value IS NULL AND source_label IS NOT NULL AND length(source_label)>0)
     OR (status<>'unresolved' AND value IS NOT NULL AND source_label IS NULL))
) STRICT;
CREATE INDEX observations_hex ON observations(hex);
CREATE TABLE observation_sources (
  observation TEXT NOT NULL REFERENCES observations(id),
  source TEXT NOT NULL REFERENCES sources(path), reference TEXT NOT NULL,
  PRIMARY KEY(observation,reference)
) STRICT;
-- One row per directed crossing, with evidence status. This is not passability.
CREATE VIEW network_links AS
SELECT o.id, o.survey, s.hex AS from_hex, s.other AS to_hex, o.value AS feature, o.status
FROM observations o JOIN hexsides s ON (o.hex=s.hex AND o.direction=s.direction)
WHERE o.kind='hexside' AND o.depiction IS NULL AND o.value IN ('road','railroad','track','pipeline')
UNION ALL
SELECT o.id, o.survey, s.other, s.hex, o.value, o.status
FROM observations o JOIN hexsides s ON (o.hex=s.hex AND o.direction=s.direction)
WHERE o.kind='hexside' AND o.depiction IS NULL AND o.value IN ('road','railroad','track','pipeline');
CREATE VIEW coverage AS
SELECT section, count(*) AS geometry_hexes,
  sum(terrain_status='partial') AS partially_transcribed_hexes,
  sum(terrain_status='unmapped') AS unmapped_hexes
FROM hexes GROUP BY section;
