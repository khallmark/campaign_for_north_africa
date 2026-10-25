-- Seventeen otherwise unchanged cells inherited row-wide uncertainty prose
-- about CPA/range that patches 001/006 have already recovered. Preserve their
-- numeric values, symbols, statuses, conditional prefixes and original notes.
CREATE TEMP TABLE note_precondition (matched INTEGER CHECK(matched=17));
INSERT INTO note_precondition SELECT count(*) FROM effective_stat
WHERE record_id IN ('weapons-ge-7-62cm-pak-r','weapons-ge-marder-iii-sp','aircraft-it-cant-z-506b-airone-standard')
AND instr(note,'unresolved') > 0;

INSERT INTO correction_evidence
SELECT 'stat-note:' || s.record_id || ':' || s.stat_id,e.source_path,e.locator,e.quote,
       replace(s.note,
         CASE s.record_id
           WHEN 'weapons-ge-7-62cm-pak-r' THEN 'Captured Russian gun. Printed row shifts left and does not give a reliable CPA cell; unresolved rather than inferred.'
           WHEN 'weapons-ge-marder-iii-sp' THEN 'Printed row shifts left and does not give a reliable CPA cell; unresolved rather than inferred.'
           ELSE 'Printed range cell appears displaced into aircraft name as /35; leave unresolved rather than interpreting as 35 or 135.' END,
         CASE s.record_id
           WHEN 'weapons-ge-7-62cm-pak-r' THEN 'Captured Russian gun. September 1979 erratum 4.49 supplies CPA 15; this other rating is unchanged.'
           WHEN 'weapons-ge-marder-iii-sp' THEN 'September 1979 erratum 4.49 supplies CPA 25; this other rating is unchanged.'
           ELSE 'September 1979 erratum 4.4b supplies range 110; this other rating is unchanged.' END)
FROM effective_stat s JOIN correction_evidence e ON e.id='record-note:' || s.record_id
WHERE instr(s.note,'unresolved') > 0;

INSERT INTO stat_correction
SELECT s.record_id,s.stat_id,s.value,s.status,s.raw,s.conditional,e.id
FROM effective_stat s JOIN correction_evidence e ON e.id='stat-note:' || s.record_id || ':' || s.stat_id;
DROP TABLE note_precondition;
