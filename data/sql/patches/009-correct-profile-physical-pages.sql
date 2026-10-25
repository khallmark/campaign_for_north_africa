-- Fresh one-based pdftoppm renders confirm the existing scan-137/scan-170
-- citations: German v is on physical p.137; Italian pp is on physical p.170.
-- Correct only the swapped locator labels introduced in 006/008. Preserve
-- the published patch ledger, quoted witnesses, observations and all ratings.
CREATE TEMP TABLE page_precondition (matched INTEGER CHECK(matched=2));
INSERT INTO page_precondition SELECT count(*) FROM correction_evidence
WHERE (id='chart-ge-v-assault-symbol' AND locator='German Unit Characteristics Chart.png, profile v shared assault column; original PDF p.170; rules.txt line 34564')
   OR (id='chart-it-pp-aa-allowance' AND locator='Italian Unit Characteristics Chart.png, profile pp maximum TOE and assignment text; original PDF p.137; rules.txt lines 34453-34456');
UPDATE correction_evidence SET locator=replace(locator,'original PDF p.170;','original PDF p.137;')
WHERE id='chart-ge-v-assault-symbol';
UPDATE correction_evidence SET locator=replace(locator,'original PDF p.137;','original PDF p.170;')
WHERE id='chart-it-pp-aa-allowance';
DROP TABLE page_precondition;
