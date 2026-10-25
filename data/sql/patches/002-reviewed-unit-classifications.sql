-- Twelve individually reviewed source rows, not profile-wide reclassification.
-- Symbol evidence was visually compared with original rule 4.22 (PDF pp7-8).
-- Quotes below are searchable anchors in the locked rules transcription; notes
-- identify the inspected VASSAL image/GPID evidence, not new canonical links.
-- Historical motorization does not establish current transport or numeric CPA.
CREATE TEMP TABLE reviewed_classification (
    record_id TEXT PRIMARY KEY, old_type TEXT NOT NULL, new_type TEXT NOT NULL,
    locator TEXT NOT NULL, quote TEXT NOT NULL, note TEXT NOT NULL
);
INSERT INTO reviewed_classification VALUES
('oa-129-041-4th-sa-armored-car-regt', 'armored-recon', 'armored-car',
 '4.22; OA physical p129 row 041', 'Armored Car',
 'VASSAL images/SA Unassigned South African Units.png identifies 4SA/profile rr; images/SA 4 SA - none.svg (GPID3466) has the armored-car symbol, not armored recon.'),
('oa-122-066-a-bn-special-service-bde-layforce', 'infantry', 'commandos',
 '27.71; OA physical p122 row 066', 'One of the Commando units in the game',
 'Rule27.71 explicitly identifies A Battalion of the Special Service Brigade (Layforce). VASSAL infantry icon does not erase the explicit commando rule.'),
('oa-122-067-special-air-service-bde', 'infantry', 'commandos',
 '27.8/27.81; OA physical p122 row 067', 'SAS Brigade may operate as a normal infantry',
 'Rule27.8 explicitly covers SAS as commandos. Normal infantry operation remains allowed; this classification changes no capabilities.'),
('oa-155-014-1st-marine-bn', 'infantry', 'marines',
 '4.22; OA physical p155 row 014', 'Marines',
 'VASSAL images/IT 1 - Marines.png (GPID5182) displays the marine anchor symbol. Correct this battalion only; do not reclassify its headquarters or all profile it-x records.'),
('oa-163-006-15th-kradschutzen-bn', 'infantry', 'motorcycle-infantry',
 '4.22/4.45c; OA physical p163 row 006', 'Kradschutzen= Motorcycle infantry',
 'VASSAL images/GE 15th Panzer Division.png maps the row to counter15/200/profile g; images/GE 15-200 - 15.png (GPID3499) displays motorcycle infantry.'),
('oa-162-099-3-288-schwinf-bn', 'heavy-weapons', 'motorized-heavy-weapons',
 '4.22; OA physical p162 row 099', 'Motorized Heavy Weapons',
 'VASSAL images/GE Sonderverband 288.png identifies row3/profile p; images/GE 3 - 288 Son III.png (GPID3522) displays motorized heavy weapons. Profile ge-p and current transport remain unchanged.'),
('oa-116-006-1st-rifle-brigade', 'infantry', 'motorized-infantry',
 '4.22; OA physical p116 row 006', 'Motorized Infantry: These are infantry units that',
 'Existing linked GPIDs3698/3925 use images/BR 1RflBde - 2-1.svg with the motorized infantry symbol. Commonwealth profile cw-l has historical motorization +; retain its base CPA and conditional note.'),
('oa-162-088-2nd-machinegewehr-bn', 'machinegun', 'motorized-machinegun',
 '4.22; OA physical p162 row 088', 'Motorized Machinegun',
 'VASSAL images/GE 2 - 5 Le.png (GPID3509) displays motorized machinegun; German profile ge-n retains its historical motorization + and unchanged numbers.'),
('oa-132-113-1st-company-fr-mtr-marine', 'infantry', 'motorized-marines',
 '4.45a; OA physical p132 row 113 note a', 'company of the Ist Free French Motor Marine Battalion.',
 'Visually checked VASSAL images/AL Unasisgned Allied Units.png note a explicitly identifies the Motor Marine company, profile v. Token link remains unresolved: AL 1 - 1 FF X.svg is artillery, not this company.'),
('oa-162-098-2-288-gebergs-inf-bn', 'infantry', 'motorized-mountain-infantry',
 '4.45c; OA physical p162 row 098 note b', 'A motorized mountain infantry',
 'VASSAL images/GE Sonderverband 288.png note b explicitly says A motorized mountain infantry unit!; images/GE 2 - 288 Son III.png (GPID3507) agrees. Preserve profile ge-m.'),
('oa-130-064-10th-nz-rr-construction-coy', 'road-construction', 'railroad-engineer',
 '4.22/4.45a; OA physical p130 row 064', 'Railroad Engineer',
 'VASSAL images/NZ Unassigned Newzealand Units.png note a explicitly expands RR as Railroad; images/NZ 10NZ - none.svg (GPID2495) displays RR. Shared profile cw-uu also covers road construction: leave the profile unchanged.'),
('oa-130-065-13th-nz-rr-construction-coy', 'road-construction', 'railroad-engineer',
 '4.22/4.45a; OA physical p130 row 065', 'Railroad Engineer',
 'VASSAL images/NZ Unassigned Newzealand Units.png note a explicitly expands RR as Railroad; images/NZ 13NZ - none.svg (GPID2499) displays RR. Do not infer or change affiliation or identity.');

-- Fail instead of silently skipping a missing, moved or already changed row.
CREATE TEMP TABLE classification_precondition (matched INTEGER CHECK(matched = 12));
INSERT INTO classification_precondition
SELECT count(*) FROM reviewed_classification p JOIN catalog_record r ON r.id=p.record_id
WHERE r.collection='entries' AND r.unit_type_id=p.old_type;

INSERT INTO correction_evidence(id,source_path,locator,quote,note)
SELECT 'classification:' || record_id, 'docs/rules.txt', locator, quote,
       old_type || ' -> ' || new_type || '. ' || note
FROM reviewed_classification;

UPDATE catalog_record
SET unit_type_id=(SELECT new_type FROM reviewed_classification WHERE record_id=catalog_record.id)
WHERE id IN (SELECT record_id FROM reviewed_classification);

DROP TABLE classification_precondition;
DROP TABLE reviewed_classification;
