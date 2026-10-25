-- Apply the remaining explicit aircraft mission erratum, not a capability
-- inferred from the aircraft name or from the Bf.109F/G variants.
CREATE TEMP TABLE mission_precondition (matched INTEGER CHECK(matched=1));
INSERT INTO mission_precondition
SELECT count(*) FROM catalog_record WHERE id='aircraft-ge-bf-109e-standard'
 AND json_extract(details_json,'$.missions')='F:day S:day D:day';

INSERT INTO correction_evidence VALUES
('mission:aircraft-ge-bf-109e-standard','docs/CNAv2.1.0/incorporated errata.txt','4.44c',
'The Bf.109E should not have "D" capability.',
'September 1979 erratum removes D (dive-bombing) capability from Bf.109E. Fighter and scramble capabilities retained; no change to F/G variants.'),
('catalog-stat-coverage','docs/rules.txt','3.5, 4.46; remaining reference-versus-scenario limits',
'the ratings of the individual TOE',
'Reviewed errata and special-unit CPA corrections are applied with original observations retained. German profile v close assault and Italian profile pp aggregate TOE remain unresolved. Equipment-defined formation ratings still require a reviewed assigned loadout; base profiles and maximum TOE are not current scenario values.');

UPDATE catalog_record SET details_json=json_set(details_json,'$.missions','F:day S:day')
WHERE id='aircraft-ge-bf-109e-standard';
DROP TABLE mission_precondition;
