-- Source-backed reference corrections, not inferred current scenario state.
-- Original observations remain immutable. Text quotations are anchors; chart
-- legend/footnote readings were also visually checked in the named images.
INSERT INTO correction_evidence VALUES
('rules-22.63-tds-cpa','docs/rules.txt','22.63–22.66',
'They have a CPA of 25 as',
'Base vehicle CPA 25. May not voluntarily exceed it or enter enemy ZOC; towing uses 20 CP (22.65). No combat ratings; not current remaining CP.'),
('rules-22.7-recovery-cpa','docs/rules.txt','22.7, with 22.63–22.66',
'The German Mobile Tank Repair Squad is treated',
'Rule 22.7 uses the Commonwealth delivery-squadron rules: base vehicle CPA 25, towing 20 CP. Repairs only German tanks; no combat ratings. Not current remaining CP.'),
('rules-27.14-lrdg-cpa','docs/rules.txt','27.11–27.16',
'Allowance of fifty (50). However, they have no',
'LRDG base CPA 50; no normal combat ratings or stacking points. Must be formed under 27.12–27.13; this does not create a deployed unit or current CP.'),
('rules-27.2-dska-cpa','docs/rules.txt','27.2, with 27.14',
'same way — as LRDG units, with two exceptions:',
'DSA/DSKA uses LRDG rules, including base CPA 50 and no normal combat ratings. Only one; formed only from German recce/armored-car TOE. Not current remaining CP.'),
('sep79-4.44c-bf110-maneuver','docs/CNAv2.1.0/incorporated errata.txt','4.44c; Bf.110 fighter configuration only',
'the Bf.110 as a fighter has a Maneuver rating of 32 only when on Night
missions. Otherwise its rating is 30.',
'Fighter configuration: maneuver 30 normally, 32 on night missions. Conditional on mission; do not apply to the separate reconnaissance configuration.'),
('chart-4.48-75-27-date','docs/rules.txt','4.48; Italian Tank and Gun Characteristics Chart.png, 75/27 ** footnote',
'Not applicable or zero',
'Anti-armor 2 applies from Operations Stage 1 of Game-Turn 63 (January 1942); before that the rating is zero. Footnote ** in the Italian weapon chart.'),
('chart-4.49-ig18-date','docs/rules.txt','4.49; German Tank and Gun Characteristics Chart.png, 7.5cm IG18 ** footnote',
'Not applicable or zero',
'Anti-armor 3 applies from Operations Stage 1 of Game-Turn 63 (January 1942); before that the rating is zero. Footnote ** in the German weapon chart.'),
('chart-4.49-tank-bar-date','docs/rules.txt','4.49; German Tank and Gun Characteristics Chart.png, tank BAR * footnote',
'S possess a BAR of 1R until thee beginning of the 1/31 Game-Turn.',
'German tanks have BAR 1R before 1/31; the printed zero is the later base adjustment, not an unconditional all-date value. Positive means right, negative left.'),
('chart-weapon-dash','docs/rules.txt','4.47–4.49; Allied, Italian and German Tank and Gun Characteristics Chart.png legends; original PDF pp.138–140',
'Not applicable or zero',
'Printed dash. The Tank and Gun Characteristics chart legends say not applicable or zero. Kept as a source dash rather than assigning an unsupported numeric zero; consult the weapon row and its footnotes. This is not a missing assigned-weapon total.'),
('chart-aircraft-dash','docs/rules.txt','34.1; aircraft chart source-dash provenance (original PDF pp.112–113,144)',
'[34.12] Certain aircraft have more than one listed',
'Printed dash in this aircraft configuration. No numeric value is transcribed from a dash; consult its chart legend and mission/configuration notes. Ground-unit weapon attachments do not explain aircraft chart cells.');

INSERT INTO stat_correction VALUES
('support-cw-tank-delivery-a','capability_points',25,'sourced','25',0,'rules-22.63-tds-cpa'),
('support-cw-tank-delivery-b','capability_points',25,'sourced','25',0,'rules-22.63-tds-cpa'),
('support-cw-tank-delivery-c','capability_points',25,'sourced','25',0,'rules-22.63-tds-cpa'),
('support-ge-tank-recovery','capability_points',25,'sourced','25',0,'rules-22.7-recovery-cpa'),
('support-cw-lrdg-1','capability_points',50,'sourced','50',0,'rules-27.14-lrdg-cpa'),
('support-cw-lrdg-2','capability_points',50,'sourced','50',0,'rules-27.14-lrdg-cpa'),
('support-ge-dska','capability_points',50,'sourced','50',0,'rules-27.2-dska-cpa'),
('aircraft-ge-bf-110-fighter','maneuver',30,'sourced','30; 32 at night',1,'sep79-4.44c-bf110-maneuver'),
('weapons-it-75-27-gun','anti_armor',2,'sourced','2**',1,'chart-4.48-75-27-date'),
('weapons-ge-7-5cm-ig18-light-infantry-gun','anti_armor',3,'sourced','3**',1,'chart-4.49-ig18-date');

-- Pz III E was already corrected by 001; do not erase its erratum evidence.
INSERT INTO stat_correction
SELECT record_id,stat_id,value,status,raw,1,'chart-4.49-tank-bar-date'
FROM stat_observation
WHERE stat_id='breakdown_shift' AND record_id IN
 ('weapons-ge-pz-i','weapons-ge-pz-ii','weapons-ge-pz-iii-h',
  'weapons-ge-pz-iii-j-special','weapons-ge-pz-iv-d','weapons-ge-pz-iv-e','weapons-ge-pz-iv-f2-special');

-- Fix the repeated unsupported interpretation everywhere in weapon/aircraft
-- cells, not only Bofors. Preserve numeric zeros and existing erratum overlays.
INSERT INTO stat_correction
SELECT s.record_id,s.stat_id,NULL,'not_applicable','-',s.conditional,
       CASE r.kind WHEN 'equipment' THEN 'chart-weapon-dash' ELSE 'chart-aircraft-dash' END
FROM stat_observation s JOIN catalog_record r ON r.id=s.record_id
WHERE r.kind IN ('equipment','aircraft_configuration') AND s.raw='-'
  AND s.status='not_applicable'
  AND NOT EXISTS (SELECT 1 FROM stat_correction c WHERE c.record_id=s.record_id AND c.stat_id=s.stat_id);
