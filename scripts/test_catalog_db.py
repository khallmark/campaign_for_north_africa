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

    def test_arrival_import_rejects_incomplete_duplicate_unreviewed_and_changed_sources_atomically(self):
        source = self.root / 'docs/CNAv2.1.0/images/fixture.png'
        source.parent.mkdir(parents=True)
        source.write_bytes(b'locked fixture source')
        transcription = self.root / 'data/sources/formation-arrivals.json'
        transcription.parent.mkdir(parents=True)
        with closing(catalog.connect(self.database)) as db:
            db.execute("INSERT INTO source_file VALUES (?,?,?)", ('docs/CNAv2.1.0/images/fixture.png', catalog.digest(source), source.stat().st_size))
            db.execute("UPDATE catalog_record SET nationality='commonwealth',details_json=? WHERE id='record'",
                       (json.dumps({'formation': 'Fixture', 'private_text': 'DO NOT EXPORT'}),))
            db.commit()
        original = {'schema_version': 1, 'sheets': [{'image': 'fixture.png', 'formation': 'Fixture',
                    'morale_raw': '+2 (0)', 'rows': [['record', None, None, None, 'No loadout printed.']]}]}
        transcription.write_text(json.dumps(original))
        self.assertTrue(self.update(lambda db, tmp: catalog.import_arrivals(db, self.root)))
        self.assertFalse(self.update(lambda db, tmp: catalog.import_arrivals(db, self.root)))
        with closing(catalog.connect(self.database, readonly=True)) as db:
            reference = catalog.formation_references(db)['record']
        self.assertEqual(reference['arrival']['morale']['value'], 0)
        self.assertIsNone(reference['arrival']['toe_strength']['value'])
        self.assertIsNone(reference['arrival_totals']['anti_air']['value'])
        self.assertNotIn('DO NOT EXPORT', json.dumps(reference))
        for mutation in ('missing', 'duplicate', 'unknown-key', 'profile', 'equipment', 'hash'):
            document = copy.deepcopy(original)
            rows = document['sheets'][0]['rows']
            if mutation == 'missing':
                rows.clear()
            elif mutation == 'duplicate':
                rows.append(rows[0])
            elif mutation == 'unknown-key':
                rows[0][0] = 'not-a-record'
            elif mutation == 'profile':
                rows[0][1] = 'ee'
            elif mutation == 'equipment':
                rows[0][2] = '3 x invented gun'
            else:
                source.write_bytes(b'changed source')
            transcription.write_text(json.dumps(document))
            before = self.database.read_bytes()
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                self.update(lambda db, tmp: catalog.import_arrivals(db, self.root))
            self.assertEqual(self.database.read_bytes(), before)

    def test_stat_projection_preserves_originals_and_only_exports_reviewed_cells(self):
        with closing(catalog.connect(self.database)) as db:
            db.executescript("""
                INSERT INTO stat_definition VALUES ('unknown',1,'Unknown','CP','Unknown','[]');
                INSERT INTO stat_observation VALUES ('record','unknown',NULL,'unresolved','?',0,'Unknown','[]');
                INSERT INTO stat_correction VALUES ('record','cpa',0,'sourced','0',1,'review');
                INSERT INTO metadata VALUES ('private_text','DO NOT EXPORT');
            """)
        original = {"schema_version": 1, "sources": [], "entries": [{
            "id": "record", "name": "Public name", "stats": {
                "cpa": {"value": 5, "status": "sourced", "raw": "5", "conditional": False,
                        "note": "original", "source_refs": []},
                "unknown": {"value": None, "status": "unresolved", "raw": "?", "conditional": False,
                            "note": "Unknown", "source_refs": []}}}], "profiles": []}
        destination = self.root / "catalog.json"
        destination.write_text(json.dumps(original))
        before, db_before = destination.read_bytes(), self.database.read_bytes()
        with self.assertRaisesRegex(ValueError, "Stat projection drift"):
            catalog.project_stats(self.database, destination, check=True)
        self.assertEqual(destination.read_bytes(), before)
        self.assertTrue(catalog.project_stats(self.database, destination))
        projected = json.loads(destination.read_text())
        stat = projected['entries'][0]['stats']['cpa']
        self.assertEqual((stat['value'], stat['raw'], stat['status'], stat['conditional']), (0, '0', 'sourced', True))
        self.assertEqual(stat['original_observation'], original['entries'][0]['stats']['cpa'])
        self.assertEqual(stat['correction_id'], 'review')
        self.assertEqual(stat['source_refs'], ['review'])
        self.assertEqual(stat['note'], 'reviewed correction')
        self.assertEqual(projected['entries'][0]['stats']['unknown'], original['entries'][0]['stats']['unknown'])
        self.assertEqual(projected['entries'][0]['name'], 'Public name')
        self.assertEqual(projected['sources'], [{'id': 'review', 'title': 'Reviewed stat correction',
                         'path': 'fixture.txt', 'section': 'row 1', 'pdf_page': None, 'lines': None,
                         'note': 'Original observation retained alongside the correction.'}])
        self.assertNotIn('DO NOT EXPORT', destination.read_text())
        self.assertNotIn('quoted source', destination.read_text())
        after = destination.read_bytes()
        self.assertFalse(catalog.project_stats(self.database, destination, check=True))
        self.assertFalse(catalog.project_stats(self.database, destination))
        self.assertEqual(destination.read_bytes(), after)
        self.assertEqual(self.database.read_bytes(), db_before)
        collision = copy.deepcopy(projected)
        collision['sources'][0]['section'] = 'different source locator'
        destination.write_text(json.dumps(collision))
        before = destination.read_bytes()
        with self.assertRaisesRegex(ValueError, 'Correction citation collision'):
            catalog.project_stats(self.database, destination)
        self.assertEqual(destination.read_bytes(), before)
        destination.write_bytes(after)
        with closing(catalog.connect(self.database)) as db:
            db.execute("UPDATE stat_correction SET value=NULL,status='not_applicable',raw='-',conditional=0")
            db.commit()
        self.assertTrue(catalog.project_stats(self.database, destination))
        stat = json.loads(destination.read_text())['entries'][0]['stats']['cpa']
        self.assertIsNone(stat['value'])
        self.assertEqual(stat['original_observation']['value'], 5)
        for broken in ({**original, 'entries': []},
                       {**original, 'entries': [{'id': 'record', 'stats': {}}]}):
            destination.write_text(json.dumps(broken))
            before = destination.read_bytes()
            with self.assertRaises(ValueError):
                catalog.project_stats(self.database, destination)
            self.assertEqual(destination.read_bytes(), before)

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
    def test_all_formation_arrivals_have_cited_loadouts_and_rule_correct_totals(self):
        document = catalog.read_json(ROOT / 'public/data/unit-catalog.json')
        records = {r['id']: r for r in document['entries']}
        sidecar = {r['id']: r for r in catalog.read_json(ROOT / 'public/data/unit-reference.json')['records']}
        with closing(catalog.connect(catalog.DATABASE, readonly=True)) as db:
            references = catalog.formation_references(db)
        self.assertEqual(len(references), 1052)
        self.assertEqual(set(references), {k for k, r in records.items() if r['kind'] in ('formation', 'counter_formation')})
        self.assertIn('1052 existing formation rows', document['coverage']['gaps'][0])
        self.assertNotIn('unresolved', json.dumps(document['coverage']).lower())
        for record_id, reference in references.items():
            self.assertEqual(records[record_id]['formation_reference'], reference)
            self.assertEqual(sidecar[record_id]['formation_reference'], reference)
            self.assertEqual(records[record_id]['stats']['capability_points']['status'], 'unresolved')
            self.assertIsNone(records[record_id]['stats']['capability_points']['value'])
        def ref(prefix):
            return next(v for k, v in references.items() if k.startswith(prefix))
        # 13th Light AA: six TOE of Bofors, not six guns and not profile max copied into AA.
        aa = ref('oa-124-002-')
        self.assertEqual(aa['arrival']['toe_strength']['value'], 6)
        self.assertEqual(aa['arrival_totals']['anti_air']['value'], 6)
        self.assertEqual(aa['arrival_totals']['anti_air']['unit'], 'Actual AA points')
        self.assertEqual(aa['components'][0]['per_toe_stats']['anti_air']['value'], 1)
        self.assertIsNone(aa['base_stats']['anti_air']['value'])
        # Asymmetric mix: two AT TOE contribute no AA; two Bofors TOE contribute two.
        mixed = ref('oa-117-024-')
        self.assertEqual(mixed['arrival']['toe_strength']['value'], 4)
        self.assertEqual(mixed['arrival_totals']['anti_air']['value'], 2)
        self.assertIsNone(mixed['arrival_totals']['anti_armor']['value'])
        self.assertTrue(mixed['arrival_totals']['anti_armor']['conditional'])
        # A conditional maximum (US tank capacity) is not a parenthesized combat rating.
        tank = ref('oa-116-003-')
        self.assertEqual(tank['arrival_totals']['anti_air']['value'], 8)  # 8 Crusaders + 2 A13
        self.assertEqual(tank['arrival_totals']['close_assault_offensive']['value'], 28)  # 8×3 + 2×2
        self.assertNotIn('armor_protection', tank['arrival_totals'])
        rifle = ref('oa-116-006-')
        self.assertEqual(rifle['base_stats']['capability_points']['raw'], '10+')
        self.assertEqual(rifle['arrival']['toe_strength']['value'], 6)
        self.assertEqual(rifle['arrival_totals']['close_assault_offensive']['value'], 12)
        self.assertEqual(rifle['arrival_totals']['close_assault_offensive']['unit'], 'Raw combat points (before ÷10)')
        self.assertEqual(ref('oa-116-001-')['arrival']['morale']['value'], 1)
        self.assertIsNone(ref('oa-116-001-')['arrival_totals']['anti_air']['value'])
        # Alphabetic note (a) is not untrained morale: only 20th Brigade arrives at zero.
        self.assertEqual(ref('oa-125-044-')['arrival']['morale']['raw'], '+1 (a)')
        self.assertEqual(ref('oa-125-044-')['arrival']['morale']['value'], 1)
        self.assertEqual(ref('oa-125-045-')['arrival']['morale']['value'], 0)
        self.assertEqual(ref('oa-125-046-')['arrival']['morale']['value'], 0)
        for prefix in ('oa-123-096-', 'oa-117-034-', 'oa-165-065-', 'oa-155-027-'):
            self.assertIsNone(ref(prefix)['arrival']['toe_strength']['value'], prefix)
            self.assertIsNone(ref(prefix)['arrival_totals']['anti_air']['value'], prefix)
        attached = ref('oa-147-013-')
        self.assertTrue(attached['arrival']['related_record_id'].startswith('oa-157-056-'))
        self.assertEqual(attached['components'], [])
        unassigned = ref('oa-157-056-')
        self.assertIsNone(unassigned['base_profile_id'])
        self.assertEqual(unassigned['arrival']['toe_strength']['value'], 4)
        self.assertEqual(unassigned['arrival_totals']['anti_air']['value'], 0)
        coast = ref('oa-155-022-')
        self.assertEqual(coast['components'][0]['land_barrage_toe'], 4)
        self.assertEqual(coast['arrival']['stats']['capability_points']['value'], 0)
        self.assertIsNone(coast['arrival_totals']['close_assault_offensive']['value'])
        # Note b limits land-target barrage to 4/2/2 artillery TOE, not all six.
        for prefix, value, expression in (('oa-155-022-', 32, '4 × 8 + 6 × 0'),
                                          ('oa-155-024-', 18, '2 × 9 + 4 × 0'),
                                          ('oa-155-025-', 30, '2 × 15 + 2 × 0')):
            barrage = ref(prefix)['arrival_totals']['barrage']
            self.assertEqual((barrage['value'], barrage['raw']), (value, expression))
            self.assertEqual(barrage['status'], 'sourced')
            self.assertTrue(barrage['conditional'])
            self.assertIn('land-target subset', barrage['note'])
        self.assertIn('attachment', attached['base_profile_note'].lower())
        stale_notes = [record_id for record_id, reference in references.items()
                       if 'unresolved' in json.dumps(reference).lower().replace('"unresolved"', '')]
        self.assertEqual(stale_notes, [])
        self.assertEqual(ref('oa-151-017-')['base_stats']['capability_points']['raw'], '10+')
        corrected = ref('oa-165-083-')
        self.assertEqual((corrected['catalog_profile_id'], corrected['base_profile_id']), ('ge-x', 'ge-w'))
        self.assertEqual(corrected['arrival_totals']['barrage']['value'], 15)
        self.assertFalse(catalog.project_formations(catalog.DATABASE, ROOT / 'public/data/unit-catalog.json', check=True))

    def test_arrival_projection_is_bounded_atomic_and_uses_effective_db_stats(self):
        document = catalog.read_json(ROOT / 'public/data/unit-catalog.json')
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / 'catalog.json'
            database = Path(directory) / 'catalog.sqlite'
            with closing(catalog.connect(catalog.DATABASE, readonly=True)) as source, closing(catalog.connect(database)) as db:
                source.backup(db)
                db.execute("UPDATE catalog_record SET details_json=json_set(details_json,'$.secret','DO NOT EXPORT')")
                db.execute("UPDATE stat_observation SET value=7 WHERE record_id='weapons-cw-heavy-aa-3-7-inch' AND stat_id='anti_air'")
                db.commit()
            target.write_text(json.dumps(document))
            before = target.read_bytes()
            with self.assertRaisesRegex(ValueError, 'Formation projection drift'):
                catalog.project_formations(database, target, check=True)
            self.assertEqual(target.read_bytes(), before)
            self.assertTrue(catalog.project_formations(database, target))
            projected = catalog.read_json(target)
            # The only allowed record mutation is this additive field.
            for old, new in zip(document['entries'], projected['entries']):
                self.assertEqual({k: v for k, v in old.items() if k != 'formation_reference'},
                                 {k: v for k, v in new.items() if k != 'formation_reference'})
            self.assertEqual(document['profiles'], projected['profiles'])
            self.assertNotIn('DO NOT EXPORT', target.read_text())
            heavy = next(r for r in projected['entries'] if r['id'].startswith('oa-124-017-'))
            self.assertEqual(heavy['formation_reference']['arrival_totals']['anti_air']['value'], 14)  # two TOE × changed DB rating
            self.assertFalse(catalog.project_formations(database, target, check=True))
            collision = copy.deepcopy(projected)
            next(s for s in collision['sources'] if s['id'].startswith('oa-arrival:'))['note'] = 'Conflicting citation'
            target.write_text(json.dumps(collision))
            before = target.read_bytes()
            with self.assertRaisesRegex(ValueError, 'Arrival citation collision'):
                catalog.project_formations(database, target)
            self.assertEqual(target.read_bytes(), before)
            projected['entries'].reverse()
            target.write_text(json.dumps(projected))
            before = target.read_bytes()
            with self.assertRaisesRegex(ValueError, 'record order/IDs'):
                catalog.project_formations(database, target)
            self.assertEqual(target.read_bytes(), before)
            # A known coastal subset cannot turn a missing second rating into
            # zero or publish the remaining artillery term as a complete total.
            with closing(catalog.connect(database)) as db:
                db.execute("DELETE FROM stat_correction WHERE record_id='weapons-it-i-light-aa' AND stat_id='barrage'")
                db.execute("UPDATE stat_observation SET value=NULL,status='unresolved',raw='?' WHERE record_id='weapons-it-i-light-aa' AND stat_id='barrage'")
                reference = catalog.formation_references(db)['oa-155-022-2nd-tobruk-cd-arty-group']
                barrage = reference['arrival_totals']['barrage']
                self.assertIsNone(barrage['value'])
                self.assertEqual(barrage['raw'], '4 × 8')
                self.assertTrue(barrage['conditional'])
                self.assertIn('not a single numeric rating', barrage['note'])

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
            patches = Path(directory) / "patches"
            patches.mkdir()
            for patch_file in (catalog.SQL / "patches").glob("*.sql"):
                if patch_file.name <= "003-reviewed-italian-profile-links.sql":
                    (patches / patch_file.name).write_bytes(patch_file.read_bytes())
            with closing(catalog.connect(catalog.DATABASE, readonly=True)) as source, closing(catalog.connect(database)) as db:
                source.backup(db)
                db.executescript("""
                    DELETE FROM sql_patch WHERE name>='003-reviewed-italian-profile-links.sql';
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
                catalog.update_database(database, SCHEMA, patches, lambda db, tmp: None)
            self.assertEqual(database.read_bytes(), before)
            with closing(catalog.connect(database)) as db:
                db.execute("UPDATE catalog_record SET profile_id=NULL WHERE id='counter-it-61-61-sirte-division-hq'")
                db.commit()
            self.assertTrue(catalog.update_database(database, SCHEMA, patches, lambda db, tmp: None))
            with closing(catalog.connect(database, readonly=True)) as db:
                for table, rows in stable.items():
                    self.assertEqual(db.execute(f"SELECT * FROM {table} ORDER BY 1,2").fetchall(), rows, table)
                self.assertEqual(db.execute("SELECT id,name,kind,side,nationality,unit_type_id,verification,details_json FROM catalog_record ORDER BY id").fetchall(), records)
                self.assertEqual(db.execute("SELECT source_id,count(*) FROM record_source WHERE source_id LIKE 'vassal-oc-it-%' GROUP BY source_id ORDER BY source_id").fetchall(),
                                 [('vassal-oc-it-61', 14), ('vassal-oc-it-62', 13), ('vassal-oc-it-63', 14)])
            before = database.read_bytes()
            self.assertFalse(catalog.update_database(database, SCHEMA, patches, lambda db, tmp: None))
            self.assertEqual(database.read_bytes(), before)

    def test_corrected_stats_reach_catalog_and_sidecar_without_inventing_scenario_values(self):
        document = json.loads((ROOT / 'public/data/unit-catalog.json').read_text())
        records = {r['id']: r for r in document['entries'] + document['profiles']}
        reference = json.loads((ROOT / 'public/data/unit-reference.json').read_text())
        sidecar = {r['id']: r for r in reference['records'] + reference['profiles']}
        self.assertEqual(len(reference['sources']), len({s['id'] for s in reference['sources']}))
        expected = {
            ('weapons-ge-pz-iii-e', 'armor_protection'): (3, False),
            ('weapons-ge-pz-iii-e', 'breakdown_shift'): (0, True),
            ('weapons-ge-7-62cm-pak-r', 'capability_points'): (15, False),
            ('weapons-ge-marder-iii-sp', 'capability_points'): (25, False),
            ('aircraft-cw-gladiator-mk-ii-standard', 'air_fuel'): (1, False),
            ('aircraft-it-cant-z-506b-airone-standard', 'air_range'): (110, False),
            ('aircraft-ge-bf-110-fighter', 'maneuver'): (30, True),
            ('aircraft-ge-bf-110-recon-load', 'maneuver'): (28, False),
            ('support-cw-lrdg-1', 'capability_points'): (50, False),
            ('support-cw-lrdg-2', 'capability_points'): (50, False),
            ('support-ge-dska', 'capability_points'): (50, False),
            ('support-ge-tank-recovery', 'capability_points'): (25, False),
            ('support-cw-tank-delivery-a', 'capability_points'): (25, False),
            ('support-cw-tank-delivery-b', 'capability_points'): (25, False),
            ('support-cw-tank-delivery-c', 'capability_points'): (25, False),
            ('weapons-it-75-27-gun', 'anti_armor'): (2, True),
            ('weapons-ge-7-5cm-ig18-light-infantry-gun', 'anti_armor'): (3, True),
        }
        for (record, key), pair in expected.items():
            stat = records[record]['stats'][key]
            self.assertEqual((stat['value'], stat['conditional']), pair, (record, key))
        self.assertIn('32 on night missions', records['aircraft-ge-bf-110-fighter']['stats']['maneuver']['note'])
        self.assertEqual(records['aircraft-ge-bf-109e-standard']['missions'], 'F:day S:day')
        self.assertEqual(sidecar['aircraft-ge-bf-109e-standard']['missions'], 'F:day S:day')
        self.assertEqual(records['aircraft-ge-bf-109f-standard']['missions'], 'F:day S:day')
        self.assertFalse(any(gap.startswith('Pz IIIE shifted') for gap in document['coverage']['gaps']))
        with closing(catalog.connect(catalog.DATABASE, readonly=True)) as db:
            for record_id, stat_id, value, status, raw, conditional, evidence in db.execute(
                    'SELECT record_id,stat_id,value,status,raw,conditional,evidence_id FROM stat_correction'):
                stat = records[record_id]['stats'][stat_id]
                projected = sidecar[record_id]['stats'][stat_id]
                for field, expected_value in [('value', value), ('status', status), ('raw', raw),
                                              ('conditional', bool(conditional)), ('correction_id', evidence)]:
                    self.assertEqual(stat[field], expected_value, (record_id, stat_id, field))
                    self.assertEqual(projected[field], expected_value, (record_id, stat_id, field))
                self.assertEqual(stat['original_observation'], projected['original_observation'])
            self.assertEqual(db.execute("SELECT count(*) FROM stat_correction WHERE evidence_id='chart-weapon-dash'").fetchone(), (208,))
        bofors = records['weapons-cw-light-aa-bofors-40mm']['stats']
        self.assertEqual(bofors['anti_air']['value'], 1)
        self.assertIsNone(bofors['barrage']['value'])
        self.assertIn('not applicable or zero', bofors['barrage']['note'])
        self.assertNotIn('not numeric zero', bofors['barrage']['note'])
        self.assertIsNone(records['cw-ee']['stats']['anti_air']['value'])
        for record, key, raw in (('ge-v', 'close_assault_offensive', '*'),
                                 ('ge-v', 'close_assault_defensive', '*'),
                                 ('it-pp', 'maximum_toe', '8AA')):
            corrected = records[record]['stats'][key]
            self.assertEqual((corrected['value'], corrected['status'], corrected['raw']),
                             (None, 'unresolved', raw))
            self.assertEqual(corrected['original_observation']['raw'], '?')
            self.assertNotIn('unresolved', corrected['note'].lower())
            page = 137 if record == 'ge-v' else 170
            self.assertIn(f'scan-{page}', corrected['source_refs'])
            citation = next(s for s in document['sources'] if s['id'] == corrected['correction_id'])
            self.assertIn(f'original PDF p.{page};', citation['section'])
        self.assertEqual(records['oa-124-002-13th-light-aa-regt']['stats']['capability_points']['status'], 'unresolved')
        self.assertFalse(catalog.project_stats(catalog.DATABASE, ROOT / 'public/data/unit-catalog.json', check=True))

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
