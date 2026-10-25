"""Fast SQLite contracts: python3 -m unittest discover -s scripts -p test_catalog_db.py.

Only stdlib, disposable fixtures, the committed database and compatibility JSON are needed.
No Cargo, source hydration, OCR, image assets or network.
"""

import copy
from contextlib import closing
import importlib.util
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("catalog_db", ROOT / "scripts/import-catalog-db.py")
catalog = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(catalog)
SCHEMA = ROOT / "data/sql/schema.sql"

# Reviewed source rows, not a profile-wide or name-based classifier.
CLASSIFICATIONS = {
    "oa-129-041-4th-sa-armored-car-regt": ("armored-recon", "armored-car"),
    "oa-122-066-a-bn-special-service-bde-layforce": ("infantry", "commandos"),
    "oa-122-067-special-air-service-bde": ("infantry", "commandos"),
    "oa-155-014-1st-marine-bn": ("infantry", "marines"),
    "oa-163-006-15th-kradschutzen-bn": ("infantry", "motorcycle-infantry"),
    "oa-162-099-3-288-schwinf-bn": ("heavy-weapons", "motorized-heavy-weapons"),
    "oa-116-006-1st-rifle-brigade": ("infantry", "motorized-infantry"),
    "oa-162-088-2nd-machinegewehr-bn": ("machinegun", "motorized-machinegun"),
    "oa-132-113-1st-company-fr-mtr-marine": ("infantry", "motorized-marines"),
    "oa-162-098-2-288-gebergs-inf-bn": ("infantry", "motorized-mountain-infantry"),
    "oa-130-064-10th-nz-rr-construction-coy": ("road-construction", "railroad-engineer"),
    "oa-130-065-13th-nz-rr-construction-coy": ("road-construction", "railroad-engineer"),
}


def source_fixture():
    slot = {
        "gpid": "7", "entry_name": "Same label", "slot_kind": "unit_counter", "side": "allied",
        "nationality_prefix": "BR", "palette": "units", "source_line": 21, "printed_name": "A",
        "basic_image": "original.svg", "palette_path": ["Formation A"], "prototype_refs": ["Units"],
        "images": [{"name": "original.svg", "exists": True, "origin": "slot", "trait_kind": "basic"}],
        "layers": [{"origin": "slot", "name": "state", "description": "raw source",
                    "initial_state": "unknown", "first_level_value": "1",
                    "levels": [{"index": 1, "image": "original.svg", "label": "fresh"}]}],
        "duplicate_palette_candidates": [],
    }
    other = {**slot, "gpid": "8", "printed_name": "B", "palette_path": ["Formation B"]}
    census = {"schema_version": 1, "summary": {"slots": 2}, "slots": [slot, other]}
    assets = {"original.svg": {"source_path": "source/original.svg", "exists": True, "sha256": "a" * 64, "size": 17}}
    return census, assets


def seed(db, _):
    catalog.import_vassal(db, *source_fixture())
    db.executescript("""
        INSERT INTO source_file VALUES ('fixture.txt', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa', 10);
        INSERT INTO correction_evidence VALUES ('review', 'fixture.txt', 'row 1', 'quoted source', 'reviewed correction');
        INSERT INTO unit_type VALUES ('infantry',0,'Infantry','{}');
        INSERT INTO stat_definition VALUES ('cpa',0,'CPA','CP','Allowance','[]');
        INSERT INTO catalog_record VALUES ('record','entries',0,'Record','formation','allied','BR','infantry',NULL,'partial','note','{}');
        INSERT INTO stat_observation VALUES ('record','cpa',5,'sourced','5',0,'original','[]');
        INSERT INTO canonical_unit VALUES ('unit-a',0,'Unit A','allied','BR','infantry','formation_candidate','provisional',NULL,'{}');
        INSERT INTO identity_redirect VALUES ('old-a','unit-a');
        INSERT INTO unit_record VALUES ('unit-a',0,'record');
        INSERT INTO generated_art VALUES ('unit-a','missing',NULL,NULL,'{}');
        INSERT INTO unit_counter VALUES ('7','unit-a','reviewed_source_link','{}');
    """)
    db.commit()


class ImportContracts(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.database = self.root / "catalog.sqlite"
        self.patches = self.root / "patches"
        self.patches.mkdir()
        catalog.update_database(self.database, SCHEMA, self.patches, seed, bootstrap=True)

    def update(self, prepare=lambda db, tmp: None, **kwargs):
        return catalog.update_database(self.database, SCHEMA, self.patches, prepare, **kwargs)

    def query(self, sql):
        with closing(catalog.connect(self.database, readonly=True)) as db:
            return db.execute(sql).fetchall()

    def test_profile_projection_is_bounded_cited_and_preserves_unknowns(self):
        with closing(catalog.connect(self.database)) as db:
            db.executescript("""
                INSERT INTO source_citation VALUES ('chart','Reviewed chart','docs/chart.png',
                    '{"section":"ID Code","private_text":"DO NOT EXPORT"}');
                INSERT INTO catalog_record VALUES ('profile','profiles',0,'Private profile name','profile','axis','IT','infantry',NULL,'verified','private','{}');
                UPDATE catalog_record SET profile_id='profile',note='Reviewed base-profile link' WHERE id='record';
                INSERT INTO record_source VALUES ('record',0,'chart');
                INSERT INTO correction_evidence VALUES ('profile-link:record','fixture.txt','row','quote','Current CPA remains unknown');
                INSERT INTO metadata VALUES ('private_text','DO NOT EXPORT');
            """)
        original = {"schema_version": 1, "sources": [], "coverage": {"gaps": []},
                    "entries": [{"id": "record", "name": "Public name", "profile_id": None,
                                 "note": "No link", "source_refs": [],
                                 "stats": {"capability_points": {"value": None, "status": "unresolved",
                                           "raw": "?", "conditional": False, "source_refs": [], "note": "No link"}}}],
                    "profiles": [{"id": "profile", "stats": {"zero": {"value": 0, "raw": "0"}}}]}
        expected = copy.deepcopy(original)
        expected["entries"][0].update(profile_id="profile", note="Reviewed base-profile link", source_refs=["chart"])
        expected["entries"][0]["stats"]["capability_points"]["note"] = "Current CPA remains unknown"
        expected["sources"] = [{"id": "chart", "title": "Reviewed chart", "path": "docs/chart.png", "section": "ID Code"}]
        destination = self.root / "unit-catalog.json"
        destination.write_text(json.dumps(original))
        before, db_before = destination.read_bytes(), self.database.read_bytes()
        with self.assertRaisesRegex(ValueError, "Profile projection drift"):
            catalog.project_profile_links(self.database, destination, check=True)
        self.assertEqual(destination.read_bytes(), before)
        self.assertTrue(catalog.project_profile_links(self.database, destination))
        self.assertEqual(json.loads(destination.read_text()), expected)
        after = destination.read_bytes()
        self.assertFalse(catalog.project_profile_links(self.database, destination, check=True))
        self.assertFalse(catalog.project_profile_links(self.database, destination))
        self.assertEqual(destination.read_bytes(), after)
        self.assertEqual(self.database.read_bytes(), db_before)
        for field in ("entries", "profiles"):
            broken = copy.deepcopy(original)
            broken[field] = []
            destination.write_text(json.dumps(broken))
            before = destination.read_bytes()
            with self.assertRaisesRegex(ValueError, "record order/IDs"):
                catalog.project_profile_links(self.database, destination)
            self.assertEqual(destination.read_bytes(), before)

    def test_type_projection_only_changes_types_and_checks_without_writing(self):
        with closing(catalog.connect(self.database)) as db:
            db.executescript("""
                INSERT INTO unit_type VALUES ('motorized-infantry',1,'Motorized infantry','{}');
                UPDATE catalog_record SET unit_type_id='motorized-infantry',name='PRIVATE DATABASE NAME';
                INSERT INTO metadata VALUES ('private_text','MUST NEVER EXPORT');
                INSERT INTO catalog_record VALUES ('other','entries',1,'Private other','formation','axis','IT','infantry',NULL,'partial','private','{}');
            """)
        original = {"schema_version": 1, "unit_types": [{"id": "infantry"}, {"id": "motorized-infantry"}],
                    "entries": [{"id": "record", "unit_type": "infantry", "name": "Public name",
                                 "stats": {"unknown": None, "zero": 0}, "source_refs": ["second", "first"]},
                                {"id": "other", "unit_type": "infantry", "stats": {}}],
                    "profiles": [], "coverage": {"complete": False}}
        destination = self.root / "unit-catalog.json"
        destination.write_text(json.dumps(original, indent=2) + "\n")
        before, db_before = destination.read_bytes(), self.database.read_bytes()
        with self.assertRaisesRegex(ValueError, "Classification projection drift"):
            catalog.project_unit_types(self.database, destination, check=True)
        self.assertEqual(destination.read_bytes(), before)
        self.assertTrue(catalog.project_unit_types(self.database, destination))
        expected = copy.deepcopy(original)
        expected["entries"][0]["unit_type"] = "motorized-infantry"
        self.assertEqual(json.loads(destination.read_text()), expected)
        projected = destination.read_bytes()
        self.assertFalse(catalog.project_unit_types(self.database, destination))
        self.assertFalse(catalog.project_unit_types(self.database, destination, check=True))
        self.assertEqual(destination.read_bytes(), projected)
        self.assertEqual(self.database.read_bytes(), db_before)
        for entries in ([], [original["entries"][0]] * 2, list(reversed(original["entries"])),
                        [{"id": "wrong", "unit_type": "infantry"}]):
            with self.subTest(entries=entries):
                destination.write_text(json.dumps({**original, "entries": entries}))
                before = destination.read_bytes()
                with self.assertRaisesRegex(ValueError, "record order/IDs"):
                    catalog.project_unit_types(self.database, destination)
                self.assertEqual(destination.read_bytes(), before)

    def test_patch_order_once_and_reimport_preserves_corrections(self):
        # Create files in reverse order: lexical numbering, not filesystem order, controls application.
        (self.patches / "002-refine.sql").write_text("UPDATE counter_correction SET label=label || ' refined' WHERE gpid='7';")
        (self.patches / "001-fix.sql").write_text("""
            INSERT INTO counter_correction VALUES ('7','Corrected','review');
            INSERT INTO stat_correction VALUES ('record','cpa',NULL,'not_applicable','-',0,'review');
        """)
        self.assertTrue(self.update())
        self.assertEqual(self.query("SELECT label,effective_label FROM effective_counter WHERE gpid='7'"), [("Same label", "Corrected refined")])
        self.assertEqual(self.query("SELECT value,original_value FROM effective_stat"), [(None, 5.0)])
        before = self.database.read_bytes()
        self.assertFalse(self.update())
        self.assertFalse(self.update(lambda db, tmp: catalog.import_vassal(db, *source_fixture())))
        self.assertEqual(before, self.database.read_bytes())
        self.assertEqual(self.query("SELECT count(*) FROM unit_counter"), [(1,)])
        self.assertEqual(self.query("SELECT target_id FROM identity_redirect"), [("unit-a",)])
        self.assertEqual(self.query("SELECT status,portrait_path FROM generated_art"), [("missing", None)])
        changed, assets = source_fixture()
        changed["slots"][0]["entry_name"] = "Revised source label"
        prepare = lambda db, tmp: catalog.import_vassal(db, changed, assets)
        with self.assertRaisesRegex(ValueError, "Database drift"):
            self.update(prepare, check=True)
        self.assertEqual(before, self.database.read_bytes())
        self.assertTrue(self.update(prepare))
        self.assertEqual(self.query("SELECT label,effective_label FROM effective_counter WHERE gpid='7'"),
                         [("Revised source label", "Corrected refined")])
        self.assertEqual(self.query("SELECT gpid,unit_id FROM unit_counter"), [("7", "unit-a")])

    def test_failed_sql_and_failed_final_validation_never_replace_database(self):
        before = self.database.read_bytes()
        patch = self.patches / "001-broken.sql"
        patch.write_text("UPDATE canonical_unit SET name='Should not survive'; SELECT * FROM nonexistent;")
        with self.assertRaises(sqlite3.Error):
            self.update()
        self.assertEqual(before, self.database.read_bytes())
        patch.write_text("UPDATE canonical_unit SET name='Reviewed name';")

        def reject(_):
            raise ValueError("Source quote mismatch")

        with self.assertRaisesRegex(ValueError, "Source quote mismatch"):
            self.update(verify=reject)
        self.assertEqual(before, self.database.read_bytes())
        self.assertTrue(self.update())
        self.assertEqual(self.query("SELECT name FROM canonical_unit"), [("Reviewed name",)])

    def test_modified_or_removed_applied_patch_is_rejected(self):
        patch = self.patches / "001-name.sql"
        original = "UPDATE canonical_unit SET name='Reviewed name';"
        patch.write_text(original)
        self.update()
        before = self.database.read_bytes()
        patch.write_text(original + "\n-- changed history")
        with self.assertRaisesRegex(ValueError, "Applied SQL patch changed"):
            self.update()
        patch.write_text(original)
        earlier = self.patches / "000-earlier.sql"
        earlier.write_text("SELECT 1;")
        with self.assertRaisesRegex(ValueError, "unchanged prefix"):
            self.update()
        earlier.unlink()
        patch.unlink()
        with self.assertRaisesRegex(ValueError, "unchanged prefix"):
            self.update()
        self.assertEqual(before, self.database.read_bytes())

    def test_corrections_cannot_overwrite_imported_observations(self):
        before = self.database.read_bytes()
        patch = self.patches / "001-correction.sql"
        for sql in ("UPDATE vassal_counter SET label='Wrong';",
                    "UPDATE stat_observation SET value=99;",
                    "DELETE FROM sql_patch;"):
            with self.subTest(sql=sql):
                patch.write_text(sql)
                with self.assertRaises(sqlite3.DatabaseError):
                    self.update()
                self.assertEqual(before, self.database.read_bytes())

    def test_redirect_collisions_and_missing_targets_never_publish(self):
        before = self.database.read_bytes()
        patch = self.patches / "001-redirect.sql"
        for sql in ("INSERT INTO identity_redirect VALUES ('unit-a','unit-a');",
                    "INSERT INTO identity_redirect VALUES ('old-b','missing');",
                    "INSERT INTO canonical_unit SELECT 'old-a',1,name,side,nationality,role_id,entity_kind,identity_status,formation_context,details_json FROM canonical_unit;"):
            with self.subTest(sql=sql):
                patch.write_text(sql)
                with self.assertRaises((sqlite3.IntegrityError, ValueError)):
                    self.update()
                self.assertEqual(before, self.database.read_bytes())

    def test_check_detects_pending_patch_without_writing(self):
        self.assertFalse(self.update(check=True))
        before = self.database.read_bytes()
        (self.patches / "001-change.sql").write_text("UPDATE canonical_unit SET name='Changed';")
        with self.assertRaisesRegex(ValueError, "Database drift"):
            self.update(check=True)
        self.assertEqual(before, self.database.read_bytes())
        with self.assertRaisesRegex(ValueError, "refuses to overwrite"):
            self.update(bootstrap=True)

    def test_private_database_cannot_be_written_into_browser_outputs(self):
        with patch.object(catalog, "ROOT", self.root):
            for name in ("public", "dist"):
                directory = self.root / name
                directory.mkdir()
                destination = directory / "catalog.sqlite"
                with self.subTest(directory=name):
                    with self.assertRaisesRegex(ValueError, "private database"):
                        catalog.update_database(destination, SCHEMA, self.patches, seed, bootstrap=True)
                    self.assertFalse(destination.exists())

    def test_missing_linked_gpid_fails_without_destroying_reviewed_identity(self):
        census, assets = source_fixture()
        census = copy.deepcopy(census)
        census["slots"] = census["slots"][1:]
        census["summary"]["slots"] = 1
        before = self.database.read_bytes()
        with self.assertRaises(sqlite3.IntegrityError):
            self.update(lambda db, tmp: catalog.import_vassal(db, census, assets))
        self.assertEqual(before, self.database.read_bytes())

    def test_schema_keeps_unknown_zero_and_art_classes_distinct(self):
        with closing(catalog.connect(self.database)) as db:
            with self.assertRaises(sqlite3.IntegrityError):
                db.execute("UPDATE stat_observation SET status='unresolved'")
            db.execute("UPDATE stat_observation SET value=0,raw='0'")
            self.assertEqual(db.execute("SELECT value,status FROM effective_stat").fetchone(), (0.0, "sourced"))
            with self.assertRaises(sqlite3.IntegrityError):
                db.execute("UPDATE generated_art SET status='ready'")
            with self.assertRaises(sqlite3.IntegrityError):
                db.execute("UPDATE token_asset SET public_path='/tokens/a.svg'")
            with self.assertRaises(sqlite3.IntegrityError):
                db.execute("UPDATE token_asset SET sha256=NULL")
            db.rollback()


class CommittedDatabaseContracts(unittest.TestCase):
    def test_reviewed_patch_changes_only_twelve_types_and_rejects_wrong_baseline(self):
        with tempfile.TemporaryDirectory() as directory:
            database = Path(directory) / "catalog.sqlite"
            patches = Path(directory) / "patches"
            patches.mkdir()
            for patch_file in (catalog.SQL / "patches").glob("*.sql"):
                if patch_file.name <= "002-reviewed-unit-classifications.sql":
                    (patches / patch_file.name).write_bytes(patch_file.read_bytes())
            with closing(catalog.connect(catalog.DATABASE, readonly=True)) as source, closing(catalog.connect(database)) as db:
                source.backup(db)
                # Reconstruct the pre-patch classifications on a disposable copy.
                db.execute("DELETE FROM sql_patch WHERE name >= '002-reviewed-unit-classifications.sql'")
                db.execute("DELETE FROM correction_evidence WHERE id LIKE 'classification:%'")
                for record_id, (old, _) in CLASSIFICATIONS.items():
                    db.execute("UPDATE catalog_record SET unit_type_id=? WHERE id=?", (old, record_id))
                db.commit()
                records = db.execute("SELECT * FROM catalog_record ORDER BY id").fetchall()
                stable_tables = ("stat_observation", "stat_correction", "canonical_unit", "identity_redirect",
                                 "unit_record", "unit_counter", "generated_art")
                stable = {table: db.execute(f"SELECT * FROM {table} ORDER BY 1,2").fetchall() for table in stable_tables}
                bad_id = "oa-130-064-10th-nz-rr-construction-coy"
                db.execute("UPDATE catalog_record SET unit_type_id='infantry' WHERE id=?", (bad_id,))
                db.commit()
            before = database.read_bytes()
            with self.assertRaises(sqlite3.IntegrityError):
                catalog.update_database(database, SCHEMA, patches, lambda db, tmp: None)
            self.assertEqual(database.read_bytes(), before)
            with closing(catalog.connect(database)) as db:
                db.execute("UPDATE catalog_record SET unit_type_id='road-construction' WHERE id=?", (bad_id,))
                db.commit()
            self.assertTrue(catalog.update_database(database, SCHEMA, patches, lambda db, tmp: None))
            with closing(catalog.connect(database, readonly=True)) as db:
                expected = [r[:7] + (CLASSIFICATIONS[r[0]][1],) + r[8:] if r[0] in CLASSIFICATIONS else r for r in records]
                self.assertEqual(db.execute("SELECT * FROM catalog_record ORDER BY id").fetchall(), expected)
                for table, rows in stable.items():
                    self.assertEqual(db.execute(f"SELECT * FROM {table} ORDER BY 1,2").fetchall(), rows, table)
                self.assertEqual(db.execute("SELECT count(*) FROM correction_evidence WHERE id LIKE 'classification:%'").fetchone(), (12,))
            before = database.read_bytes()
            self.assertFalse(catalog.update_database(database, SCHEMA, patches, lambda db, tmp: None))
            self.assertEqual(database.read_bytes(), before)

    def test_profile_patch_preserves_stats_identity_and_rejects_wrong_baseline(self):
        with tempfile.TemporaryDirectory() as directory:
            database = Path(directory) / "catalog.sqlite"
            with closing(catalog.connect(catalog.DATABASE, readonly=True)) as source, closing(catalog.connect(database)) as db:
                source.backup(db)
                db.executescript("""
                    DELETE FROM sql_patch WHERE name='003-reviewed-italian-profile-links.sql';
                    UPDATE catalog_record SET profile_id=NULL,note='Original counter-only identity'
                        WHERE id IN (SELECT substr(id,14) FROM correction_evidence WHERE id LIKE 'profile-link:%');
                    DELETE FROM correction_evidence WHERE id LIKE 'profile-link:%';
                    DELETE FROM record_source WHERE source_id LIKE 'vassal-oc-it-%';
                    DELETE FROM source_citation WHERE id LIKE 'vassal-oc-it-%';
                """)
                stable_tables = ("stat_observation", "stat_correction", "canonical_unit", "identity_redirect",
                                 "unit_record", "unit_counter", "generated_art")
                stable = {table: db.execute(f"SELECT * FROM {table} ORDER BY 1,2").fetchall() for table in stable_tables}
                records = db.execute("SELECT id,name,kind,side,nationality,unit_type_id,verification,details_json FROM catalog_record ORDER BY id").fetchall()
                db.execute("UPDATE catalog_record SET profile_id='it-h' WHERE id='counter-it-61-61-sirte-division-hq'")
                db.commit()
            before = database.read_bytes()
            with self.assertRaises(sqlite3.IntegrityError):
                catalog.update_database(database, SCHEMA, catalog.SQL / "patches", lambda db, tmp: None)
            self.assertEqual(database.read_bytes(), before)
            with closing(catalog.connect(database)) as db:
                db.execute("UPDATE catalog_record SET profile_id=NULL WHERE id='counter-it-61-61-sirte-division-hq'")
                db.commit()
            self.assertTrue(catalog.update_database(database, SCHEMA, catalog.SQL / "patches", lambda db, tmp: None))
            with closing(catalog.connect(database, readonly=True)) as db:
                for table, rows in stable.items():
                    self.assertEqual(db.execute(f"SELECT * FROM {table} ORDER BY 1,2").fetchall(), rows, table)
                self.assertEqual(db.execute("SELECT id,name,kind,side,nationality,unit_type_id,verification,details_json FROM catalog_record ORDER BY id").fetchall(), records)
                self.assertEqual(db.execute("SELECT source_id,count(*) FROM record_source WHERE source_id LIKE 'vassal-oc-it-%' GROUP BY source_id ORDER BY source_id").fetchall(),
                                 [('vassal-oc-it-61', 14), ('vassal-oc-it-62', 13), ('vassal-oc-it-63', 14)])
            before = database.read_bytes()
            self.assertFalse(catalog.update_database(database, SCHEMA, catalog.SQL / "patches", lambda db, tmp: None))
            self.assertEqual(database.read_bytes(), before)

    def test_reviewed_profiles_are_numeric_cited_and_projected_to_sidecar(self):
        document = json.loads((ROOT / "public/data/unit-catalog.json").read_text())
        entries = {r['id']: r for r in document['entries']}
        # Explicit chart cells span all eight codes and all three divisions.
        examples = {
            'counter-it-61-61-sirte-division-hq': 'it-b',
            'counter-it-61-69-infantry-regiment-hq': 'it-h',
            'counter-it-62-ii-115-infantry-battalion': 'it-u',
            'counter-it-63-63-machinegun-battalion': 'it-bb',
            'counter-it-62-62-anti-tank-company': 'it-rr',
            'counter-it-63-45-artillery-regiment': 'it-kk',
            'counter-it-61-61-engineer-battalion': 'it-bbb',
            'counter-it-63-63-bersaglieri-company': 'it-aaa',
        }
        for record_id, profile_id in examples.items():
            self.assertEqual(entries[record_id]['profile_id'], profile_id)
        profiles = {r['id']: r for r in document['profiles']}
        reference = json.loads((ROOT / 'public/data/unit-reference.json').read_text())
        with closing(catalog.connect(catalog.DATABASE, readonly=True)) as db:
            linked = dict(db.execute("SELECT r.id,r.profile_id FROM catalog_record r JOIN correction_evidence e ON e.id='profile-link:' || r.id"))
            self.assertEqual(len(linked), 41)
            for record_id, profile_id in linked.items():
                entry = entries[record_id]
                self.assertEqual(entry['profile_id'], profile_id)
                self.assertIsNone(entry['stats']['capability_points']['value'])
                self.assertEqual(entry['stats']['capability_points']['status'], 'unresolved')
                self.assertTrue(any(s['value'] is not None for s in profiles[profile_id]['stats'].values()))
                self.assertEqual(entry['source_refs'], [r[0] for r in db.execute(
                    'SELECT source_id FROM record_source WHERE record_id=? ORDER BY position', (record_id,))])
            for record in reference['records']:
                self.assertEqual(record['profile_id'], entries[record['id']].get('profile_id'))
            for unit in reference['units']:
                expected = sorted({entries[r]['profile_id'] for r in unit['source_record_ids'] if entries[r].get('profile_id')})
                self.assertEqual(unit['characteristic_profile_ids'], expected)
        self.assertFalse(catalog.project_profile_links(catalog.DATABASE, ROOT / 'public/data/unit-catalog.json', check=True))

    def test_classifications_and_compatibility_projection_agree(self):
        with closing(catalog.connect(catalog.DATABASE, readonly=True)) as db:
            for record_id, (_, expected) in CLASSIFICATIONS.items():
                with self.subTest(record_id=record_id):
                    self.assertEqual(db.execute("SELECT unit_type_id FROM catalog_record WHERE id=?", (record_id,)).fetchone(), (expected,))
            # Shared profiles and unresolved token links are not classification corrections.
            self.assertEqual(db.execute("SELECT unit_type_id FROM catalog_record WHERE id='cw-uu'").fetchone(), ("road-construction",))
            self.assertEqual(db.execute("SELECT count(*) FROM unit_counter WHERE unit_id='unit-commonwealth-1st-company-fr-mtr-marine'").fetchone(), (0,))
            self.assertFalse(catalog.project_unit_types(catalog.DATABASE, ROOT / "public/data/unit-catalog.json", check=True))
            reference = json.loads((ROOT / "public/data/unit-reference.json").read_text())
            types = dict(db.execute("SELECT id,unit_type_id FROM catalog_record"))
            for record in reference["records"] + reference["profiles"]:
                self.assertEqual(record["unit_type"], types[record["id"]])
            for unit in reference["units"]:
                self.assertEqual(unit["unit_type_ids"], sorted({types[r] for r in unit["source_record_ids"]}))

    def test_committed_database_identity_assets_stats_and_patch_ledger(self):
        self.assertTrue(catalog.DATABASE.is_file(), "Committed database is required; do not skip this test")
        with closing(catalog.connect(catalog.DATABASE, readonly=True)) as db:
            catalog.validate(db)
            self.assertEqual(db.execute("SELECT canonical_candidates,vassal_slots,generated_art_ready,complete FROM coverage").fetchone(), (1356, 2468, 61, 0))
            self.assertEqual(db.execute("SELECT count(*) FROM identity_redirect").fetchone()[0], 18)
            self.assertEqual(db.execute("SELECT count(*) FROM unit_token WHERE unit_id='unit-commonwealth-2nd-bays'").fetchone()[0], 2)
            self.assertEqual(db.execute("SELECT value,original_value FROM effective_stat WHERE record_id='weapons-ge-7-62cm-pak-r' AND stat_id='capability_points'").fetchone(), (15.0, None))
            self.assertEqual(db.execute("SELECT value FROM effective_stat WHERE record_id='it-pp' AND stat_id='capability_points'").fetchone(), (0.0,))
            self.assertEqual(db.execute("SELECT count(*) FROM token_asset WHERE present=0 AND sha256 IS NULL AND private_review_path IS NULL").fetchone()[0], 14)
            self.assertEqual(db.execute("SELECT count(*) FROM token_asset WHERE public_path IS NOT NULL").fetchone()[0], 0)
            self.assertEqual(db.execute("SELECT count(*) FROM source_document").fetchone()[0], 262)
            ledger = dict(db.execute("SELECT name,sha256 FROM sql_patch"))
            self.assertEqual(ledger, {p.name: catalog.digest(p) for p in (catalog.SQL / "patches").glob("*.sql")})


if __name__ == "__main__":
    unittest.main()
