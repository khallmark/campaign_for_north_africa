-- Correction quotes are mechanically checked against locked text, not decoded
-- image bytes. Keep the visually checked chart and physical page in the locator;
-- the matching locked OCR line is a corroborating witness, not a numeric guess.
UPDATE correction_evidence SET source_path='docs/rules.txt',quote='Vv 20 - - - - - * S',
locator='German Unit Characteristics Chart.png, profile v shared assault column; original PDF p.170; rules.txt line 34564'
WHERE id='chart-ge-v-assault-symbol';
UPDATE correction_evidence SET source_path='docs/rules.txt',quote='Strength Points 8AA',
locator='Italian Unit Characteristics Chart.png, profile pp maximum TOE and assignment text; original PDF p.137; rules.txt lines 34453-34456'
WHERE id='chart-it-pp-aa-allowance';
