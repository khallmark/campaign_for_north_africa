"""Fast SQLite contracts: python3 -m unittest discover -s scripts -p test_catalog_db.py.

Only stdlib, disposable synthetic fixtures and the committed database are needed.
No Cargo, source hydration, OCR, image assets, network or existing JSON exports.
"""

import copy
from contextlib import closing
import importlib.util
from pathlib import Path
import sqlite3
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("catalog_db", ROOT / "scripts/import-catalog-db.py")
catalog = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(catalog)
SCHEMA = ROOT / "data/sql/schema.sql"


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
