-- Individually transcribed ID-code cells from the three VASSAL OC charts.
-- These are links to base characteristics, not current/scenario ratings.
-- Retain original counter observations, canonical IDs, affiliations and artwork.
CREATE TEMP TABLE reviewed_profile_link (
    record_id TEXT PRIMARY KEY, profile_id TEXT NOT NULL, source_id TEXT NOT NULL,
    printed_row TEXT NOT NULL
);
INSERT INTO reviewed_profile_link VALUES
('counter-it-61-61-sirte-division-hq','it-b','vassal-oc-it-61','61st Sirte Div HQ: b'),
('counter-it-61-69-infantry-regiment-hq','it-h','vassal-oc-it-61','69th Infantry Regt HQ: h'),
('counter-it-61-i-69-infantry-battalion','it-u','vassal-oc-it-61','I/69 Infantry Bn: u'),
('counter-it-61-ii-69-infantry-battalion','it-u','vassal-oc-it-61','II/69 Infantry Bn: u'),
('counter-it-61-iii-69-infantry-battalion','it-u','vassal-oc-it-61','III/69 Infantry Bn: u'),
('counter-it-61-70-infantry-regiment-hq','it-h','vassal-oc-it-61','70th Infantry Regt HQ: h'),
('counter-it-61-i-70-infantry-battalion','it-u','vassal-oc-it-61','I/70 Infantry Bn: u'),
('counter-it-61-ii-70-infantry-battalion','it-u','vassal-oc-it-61','II/70 Infantry Bn: u'),
('counter-it-61-iii-70-infantry-battalion','it-u','vassal-oc-it-61','III/70 Infantry Bn: u'),
('counter-it-61-61-machinegun-battalion','it-bb','vassal-oc-it-61','61st Machinegun Bn: bb'),
('counter-it-61-61-anti-tank-company','it-rr','vassal-oc-it-61','61st Anti-tank Coy: rr'),
('counter-it-61-43-artillery-regiment','it-kk','vassal-oc-it-61','43rd Artillery Regt: kk'),
('counter-it-61-61-engineer-battalion','it-bbb','vassal-oc-it-61','61st Engineer Bn: bbb'),
('counter-it-61-61-bersaglieri-company','it-aaa','vassal-oc-it-61','61st Bersaglieri Mitrg Coy: aaa'),
('counter-it-62-62-marmarica-division-hq','it-b','vassal-oc-it-62','62nd Marmarica Div HQ: b'),
('counter-it-62-115-infantry-regiment-hq','it-h','vassal-oc-it-62','115th Infantry Regt HQ: h'),
('counter-it-62-i-115-infantry-battalion','it-u','vassal-oc-it-62','I/115 Infantry Bn: u'),
('counter-it-62-ii-115-infantry-battalion','it-u','vassal-oc-it-62','II/115 Infantry Bn: u'),
('counter-it-62-iii-115-infantry-battalion','it-u','vassal-oc-it-62','III/115 Infantry Bn: u'),
('counter-it-62-116-infantry-regiment-hq','it-h','vassal-oc-it-62','116th Infantry Regt HQ: h'),
('counter-it-62-i-116-infantry-battalion','it-u','vassal-oc-it-62','I/116 Infantry Bn: u'),
('counter-it-62-ii-116-infantry-battalion','it-u','vassal-oc-it-62','II/116 Infantry Bn: u'),
('counter-it-62-iii-116-infantry-battalion','it-u','vassal-oc-it-62','III/116 Infantry Bn: u'),
('counter-it-62-62-machinegun-battalion','it-bb','vassal-oc-it-62','62nd Machinegun Bn: bb'),
('counter-it-62-62-anti-tank-company','it-rr','vassal-oc-it-62','62nd Anti-tank Coy: rr'),
('counter-it-62-44-artillery-regiment','it-kk','vassal-oc-it-62','44th Artillery Regt: kk'),
('counter-it-62-62-engineer-battalion','it-bbb','vassal-oc-it-62','62nd Engineer Bn: bbb'),
('counter-it-63-63-cirene-division-hq','it-b','vassal-oc-it-63','63rd Cirene Div HQ: b'),
('counter-it-63-157-infantry-regiment-hq','it-h','vassal-oc-it-63','157th Infantry Regt HQ: h'),
('counter-it-63-i-157-infantry-battalion','it-u','vassal-oc-it-63','I/157 Infantry Bn: u'),
('counter-it-63-ii-157-infantry-battalion','it-u','vassal-oc-it-63','II/157 Infantry Bn: u'),
('counter-it-63-iii-157-infantry-battalion','it-u','vassal-oc-it-63','III/157 Infantry Bn: u'),
('counter-it-63-158-infantry-regiment-hq','it-h','vassal-oc-it-63','158th Infantry Regt HQ: h'),
('counter-it-63-i-158-infantry-battalion','it-u','vassal-oc-it-63','I/158 Infantry Bn: u'),
('counter-it-63-ii-158-infantry-battalion','it-u','vassal-oc-it-63','II/158 Infantry Bn: u'),
('counter-it-63-iii-158-infantry-battalion','it-u','vassal-oc-it-63','III/158 Infantry Bn: u'),
('counter-it-63-63-machinegun-battalion','it-bb','vassal-oc-it-63','63rd Machinegun Bn: bb'),
('counter-it-63-63-anti-tank-company','it-rr','vassal-oc-it-63','63rd Anti-tank Coy: rr'),
('counter-it-63-45-artillery-regiment','it-kk','vassal-oc-it-63','45th Artillery Regt: kk'),
('counter-it-63-63-engineer-battalion','it-bbb','vassal-oc-it-63','63rd Engineer Bn: bbb'),
('counter-it-63-63-bersaglieri-company','it-aaa','vassal-oc-it-63','63rd Bersaglieri Mitrg Coy: aaa');

CREATE TEMP TABLE profile_precondition (matched INTEGER CHECK(matched=41));
INSERT INTO profile_precondition
SELECT count(*) FROM reviewed_profile_link p
JOIN catalog_record r ON r.id=p.record_id
JOIN catalog_record profile ON profile.id=p.profile_id
WHERE r.collection='entries' AND r.kind='counter_formation' AND r.profile_id IS NULL
  AND profile.collection='profiles' AND profile.nationality='italian';

INSERT INTO source_citation VALUES
('vassal-oc-it-61','VASSAL 2.1.0: 61st Sirte organization chart',
 'docs/CNAv2.1.0/images/OC IT 61st Sirte Division.png',
 '{"section":"ID Code column","note":"Visually transcribed named rows; links to Italian base characteristic profiles, not current scenario ratings."}'),
('vassal-oc-it-62','VASSAL 2.1.0: 62nd Marmarica organization chart',
 'docs/CNAv2.1.0/images/OC IT 62nd Marmarica Division.png',
 '{"section":"ID Code column","note":"Visually transcribed named rows; the attached LXII(L) tank row has no printed ID code and is not inferred."}'),
('vassal-oc-it-63','VASSAL 2.1.0: 63rd Cirene organization chart',
 'docs/CNAv2.1.0/images/OC IT 63rd Cirene Division.png',
 '{"section":"ID Code column","note":"Visually transcribed named rows; the attached LXIII(L) tank row has no printed ID code and is not inferred."}');

-- Rule 3.5 is the text anchor explaining characteristics, NOT evidence of a
-- particular mapping. The inspected chart and exact printed row identify that.
INSERT INTO correction_evidence(id,source_path,locator,quote,note)
SELECT 'profile-link:' || p.record_id, 'docs/rules.txt',
       '3.5; visually reviewed ' || s.path || '; ' || p.printed_row,
       'Each counter comprises a number of TOE',
       'Counter-origin identity. Reviewed VASSAL ID-code row: ' || p.printed_row ||
       '. Base characteristics are in profile ' || p.profile_id ||
       '; current/scenario CPA remains unresolved. Original counter numerals are stacking values, not CPA.'
FROM reviewed_profile_link p JOIN source_citation s ON s.id=p.source_id;

UPDATE catalog_record
SET profile_id=(SELECT profile_id FROM reviewed_profile_link WHERE record_id=catalog_record.id),
    note=(SELECT note FROM correction_evidence WHERE id='profile-link:' || catalog_record.id)
WHERE id IN (SELECT record_id FROM reviewed_profile_link);

INSERT INTO record_source(record_id,position,source_id)
SELECT p.record_id, (SELECT coalesce(max(position),-1)+1 FROM record_source WHERE record_id=p.record_id), p.source_id
FROM reviewed_profile_link p;

DROP TABLE profile_precondition;
DROP TABLE reviewed_profile_link;
