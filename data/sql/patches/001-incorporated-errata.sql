-- Reviewed VASSAL incorporated errata. Preserve stat_observation unchanged.
-- Applies once, after VASSAL import. Never edit a patch already in sql_patch;
-- add the next numbered patch instead. No inferred corrections to other variants.
INSERT INTO correction_evidence VALUES
('sep79-4.49-pz-iii-e', 'docs/CNAv2.1.0/incorporated errata.txt', '[4.49]',
'The correct values for the German PzIIIE
are 25 1 - 4 - 3 4/4 3 0 (reading across).',
'VASSAL corrected chart agrees. Original manual row is shifted/degraded; original observations remain in source records. BAR1R until1/31 still applies; zero is later base BAR, not all dates.'),
('sep79-4.49-pak-cpa', 'docs/CNAv2.1.0/incorporated errata.txt', '[4.49]',
'The CPA for the 7.62cm Pak(R) is "15";for the
Marder, "25".',
'Original CPA unresolved. VASSAL chart agrees with errata CPA; other VASSAL row columns visibly misaligned, so only CPA is corrected here.'),
('sep79-4.49-marder-cpa', 'docs/CNAv2.1.0/incorporated errata.txt', '[4.49]',
'The CPA for the 7.62cm Pak(R) is "15";for the
Marder, "25".',
'Original CPA unresolved. VASSAL chart agrees with errata CPA; other VASSAL row columns visibly misaligned, so only CPA is corrected here.'),
('sep79-4.44a-gladiator-fuel', 'docs/CNAv2.1.0/incorporated errata.txt', '[4.44a]',
'The Fuel Consumption rating of the Gladiator is "1".',
'Errata resolves omitted original cell; does not claim the original chart printed1. Not applied to a different aircraft variant by name similarity.'),
('sep79-4.4b-z506-range', 'docs/CNAv2.1.0/incorporated errata.txt', '[4.4b]',
'The Range for the Z506 is 110.',
'Errata resolves omitted original cell; does not claim the original chart printed110.');

INSERT INTO stat_correction(record_id,stat_id,value,status,raw,conditional,evidence_id) VALUES
('weapons-ge-pz-iii-e','capability_points',25,'sourced','25',0,'sep79-4.49-pz-iii-e'),
('weapons-ge-pz-iii-e','anti_air',1,'sourced','1',0,'sep79-4.49-pz-iii-e'),
('weapons-ge-pz-iii-e','barrage',NULL,'not_applicable','-',0,'sep79-4.49-pz-iii-e'),
('weapons-ge-pz-iii-e','anti_armor',4,'sourced','4',0,'sep79-4.49-pz-iii-e'),
('weapons-ge-pz-iii-e','vulnerability',NULL,'not_applicable','-',0,'sep79-4.49-pz-iii-e'),
('weapons-ge-pz-iii-e','armor_protection',3,'sourced','3',0,'sep79-4.49-pz-iii-e'),
('weapons-ge-pz-iii-e','close_assault_offensive',4,'sourced','4',0,'sep79-4.49-pz-iii-e'),
('weapons-ge-pz-iii-e','close_assault_defensive',4,'sourced','4',0,'sep79-4.49-pz-iii-e'),
('weapons-ge-pz-iii-e','fuel_rate',3,'sourced','3',0,'sep79-4.49-pz-iii-e'),
('weapons-ge-pz-iii-e','breakdown_shift',0,'sourced','0',1,'sep79-4.49-pz-iii-e'),
('weapons-ge-7-62cm-pak-r','capability_points',15,'sourced','15',0,'sep79-4.49-pak-cpa'),
('weapons-ge-marder-iii-sp','capability_points',25,'sourced','25',0,'sep79-4.49-marder-cpa'),
('aircraft-cw-gladiator-mk-ii-standard','air_fuel',1,'sourced','1',0,'sep79-4.44a-gladiator-fuel'),
('aircraft-it-cant-z-506b-airone-standard','air_range',110,'sourced','110',0,'sep79-4.4b-z506-range');
