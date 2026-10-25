-- Preserve immutable source observations while recovering printed nonnumeric
-- symbols. The initial schema stays digest-locked; migrate the overlay only.
DROP VIEW effective_stat;
ALTER TABLE stat_correction RENAME TO previous_stat_correction;
CREATE TABLE stat_correction (
    record_id TEXT NOT NULL, stat_id TEXT NOT NULL, value REAL,
    status TEXT NOT NULL CHECK(status IN ('sourced','not_applicable','unresolved')),
    raw TEXT NOT NULL, conditional INTEGER NOT NULL CHECK(conditional IN (0,1)),
    evidence_id TEXT NOT NULL REFERENCES correction_evidence(id),
    PRIMARY KEY(record_id, stat_id),
    FOREIGN KEY(record_id, stat_id) REFERENCES stat_observation(record_id, stat_id),
    CHECK((status = 'sourced' AND value IS NOT NULL) OR
          (status IN ('not_applicable','unresolved') AND value IS NULL))
) STRICT;
INSERT INTO stat_correction SELECT * FROM previous_stat_correction;
DROP TABLE previous_stat_correction;
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

INSERT INTO correction_evidence VALUES
('chart-ge-v-assault-symbol','docs/CNAv2.1.0/images/German Unit Characteristics Chart.png',
'Profile v; shared close-assault column; original manual PDF p.170', '*',
'The shared assault cell prints *. The asterisk note describes HQ anti-tank augmentation under 19.8, not a numeric assault rating. No single assault number is supplied here; use the explicitly assigned weapons for formation arrival calculations.'),
('chart-it-pp-aa-allowance','docs/CNAv2.1.0/images/Italian Unit Characteristics Chart.png',
'Profile pp; maximum TOE column and assignment text; original manual PDF p.137', '8AA',
'Printed 8AA permits up to eight anti-air TOE Strength Points in addition to the emplaced guns. It is not an aggregate maximum for the formation. Use the OA row for the actual historical artillery and AA assignment.');
INSERT INTO stat_correction VALUES
('ge-v','close_assault_offensive',NULL,'unresolved','*',0,'chart-ge-v-assault-symbol'),
('ge-v','close_assault_defensive',NULL,'unresolved','*',0,'chart-ge-v-assault-symbol'),
('it-pp','maximum_toe',NULL,'unresolved','8AA',0,'chart-it-pp-aa-allowance');

-- Current explanations must acknowledge existing numeric errata. Originals
-- (including their old uncertainty notes) remain unchanged in stat_observation.
UPDATE correction_evidence SET note=
'Original chart CPA cell is displaced. VASSAL chart agrees with the cited erratum; only CPA is corrected, not other misaligned row columns.'
WHERE id IN ('sep79-4.49-pak-cpa','sep79-4.49-marder-cpa');
INSERT INTO correction_evidence
SELECT 'record-note:' || c.record_id,e.source_path,e.locator,e.quote,
       CASE c.record_id
         WHEN 'weapons-ge-7-62cm-pak-r' THEN 'Captured Russian gun. September 1979 erratum 4.49 supplies CPA 15.'
         WHEN 'weapons-ge-marder-iii-sp' THEN 'September 1979 erratum 4.49 supplies CPA 25.'
         ELSE 'September 1979 erratum 4.4b supplies range 110.' END ||
       ' Printed equipment/configuration, not a field formation or scenario unit. Shared illustration is not type-exact.'
FROM stat_correction c JOIN correction_evidence e ON e.id=c.evidence_id
WHERE c.evidence_id IN ('sep79-4.49-pak-cpa','sep79-4.49-marder-cpa','sep79-4.4b-z506-range');
UPDATE catalog_record SET note=(SELECT e.note FROM correction_evidence e WHERE e.id='record-note:' || catalog_record.id)
WHERE EXISTS (SELECT 1 FROM correction_evidence e WHERE e.id='record-note:' || catalog_record.id);

UPDATE correction_evidence SET note=replace(note,
'current/scenario CPA remains unresolved', 'current CPA depends on scenario state and actions already spent')
WHERE id LIKE 'profile-link:%';
UPDATE catalog_record SET note=replace(note,
'current/scenario CPA remains unresolved', 'current CPA depends on scenario state and actions already spent')
WHERE id IN (SELECT substr(id,14) FROM correction_evidence WHERE id LIKE 'profile-link:%');

UPDATE correction_evidence SET note=
'Reviewed errata and source symbols are applied with original observations retained. German profile v prints * rather than a numeric assault rating; Italian profile pp prints 8AA, an AA allowance rather than a whole-unit maximum. Historical formation totals use only quantified OA assignments and cited restrictions, never current scenario strength.'
WHERE id='catalog-stat-coverage';
INSERT INTO correction_evidence VALUES
('catalog-stat-schema-note','docs/rules.txt','3.5, 4.46; source versus scenario values',
'the ratings of the individual TOE',
'Values requiring scenario state or lacking a single source number stay null, never an invented zero.');
