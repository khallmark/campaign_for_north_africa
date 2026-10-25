"""Public SQLite import contract; source image verification runs via --write/--check."""
import importlib.util
from pathlib import Path
import sqlite3
import subprocess
import json
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("map_db", ROOT / "scripts/import-map-db.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class MapDatabaseTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.model = json.loads(subprocess.check_output(
            ["cargo", "run", "--quiet", "--locked", "-p", "cna-map", "--bin", "map-model"], cwd=ROOT,
        ))

    def setUp(self):
        self.db = sqlite3.connect(":memory:")
        self.addCleanup(self.db.close)
        module.populate(self.db, self.model)

    def test_geometry_unknowns_provenance_and_connected_networks(self):
        self.assertEqual(self.db.execute("SELECT count(*) FROM hexes").fetchone()[0], 7180)
        self.assertEqual(self.db.execute("SELECT count(*) FROM hexsides").fetchone()[0], 20684)
        self.assertEqual(self.db.execute("PRAGMA integrity_check").fetchone()[0], "ok")
        self.assertEqual(self.db.execute("PRAGMA foreign_key_check").fetchall(), [])
        self.assertEqual(self.db.execute("SELECT terrain_status FROM hexes WHERE id='A4829'").fetchone()[0], "unmapped")
        self.assertEqual(self.db.execute("SELECT value FROM observations WHERE survey='matruh-port-location'").fetchone()[0], None)
        self.assertEqual(self.db.execute("""
            SELECT o.id FROM observations o LEFT JOIN observation_sources os ON os.observation=o.id
            LEFT JOIN sources s ON os.source=s.path GROUP BY o.id
            HAVING count(os.reference)<2 OR min(length(s.sha256))<>64
        """).fetchall(), [])
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute("UPDATE observations SET source_label=NULL WHERE survey='matruh-port-location'")
        for feature in ("road", "railroad"):
            reachable = {row[0] for row in self.db.execute("""
                WITH RECURSIVE reach(hex) AS (
                  VALUES ('D3714') UNION SELECT n.to_hex FROM network_links n JOIN reach r ON n.from_hex=r.hex
                  WHERE n.feature=? AND n.status='provisional'
                    AND n.survey IN ('matruh-road-east','matruh-rail-east')
                ) SELECT hex FROM reach
            """, (feature,))}
            self.assertIn("D3519", reachable)
            self.assertNotIn("E3519", reachable)
            self.assertEqual("D3616" in reachable, feature == "road")
            self.assertEqual("D3516" in reachable, feature == "railroad")
        # TEC's disconnected crosses are unfinished railroad, not usable links.
        self.assertEqual(self.db.execute("SELECT count(*) FROM network_links WHERE survey='matruh-rail-west'").fetchone()[0], 0)
        self.assertEqual(self.db.execute("SELECT count(*) FROM observations WHERE survey='matruh-rail-west' AND status='unresolved'").fetchone()[0], 4)
        self.assertEqual(self.db.execute("SELECT count(*) FROM network_links WHERE status='reviewed'").fetchone()[0], 0)

    def test_whole_map_evidence_preserves_uncertainty_and_source_controls(self):
        self.assertEqual(self.db.execute("SELECT count(*) FROM cell_measurements").fetchone()[0], 7180)
        self.assertEqual(self.db.execute("SELECT count(*) FROM side_examinations").fetchone()[0], 20684)
        self.assertEqual(self.db.execute("SELECT count(*) FROM examinations").fetchone()[0], 35900)
        self.assertEqual(self.db.execute("""
            SELECT section,family FROM examination_coverage GROUP BY section,family
            HAVING sum(hexes)<>(SELECT count(*) FROM hexes h WHERE h.section=examination_coverage.section)
        """).fetchall(), [])
        self.assertEqual(self.db.execute("SELECT count(*) FROM observations WHERE status='reviewed'").fetchone()[0], 0)
        # A blue dot is not permission to infer a Village/Bir water rule.
        self.assertEqual(self.db.execute("SELECT status,value FROM observations WHERE survey='label-B3406'").fetchone(), ('unresolved', None))
        self.assertIn('Benina', self.db.execute("SELECT source_label FROM observations WHERE survey='label-A4829'").fetchone()[0])
        self.assertEqual(self.db.execute("SELECT value FROM observations WHERE survey='malta-luqa'").fetchone()[0], '{"type":"airfield"}')
        # Independently inspected TEC controls: pale gaps are unfinished, dark
        # grid interruptions are not. Check all surveys, not only the manual one.
        for hex_id, direction, feature, present in (
            ('D3514', 'east', 'railroad', False),
            ('C3226', 'north_west', 'railroad', False),
            ('D2224', 'north_east', 'railroad', False),  # Escarpment fringe, not ties.
            ('E2520', 'east', 'railroad', True),
            ('B5409', 'east', 'road', True),
            ('B5405', 'east', 'road', True),
            ('B5404', 'east', 'road', True),
            ('C4320', 'east', 'road', True),
            ('C4020', 'east', 'road', True),
            ('D3710', 'east', 'road', True),
            ('D3512', 'north_east', 'road', False),
        ):
            found = self.db.execute("SELECT 1 FROM observations WHERE hex=? AND direction=? AND value=?",
                                    (hex_id, direction, feature)).fetchone() is not None
            self.assertEqual(found, present, (hex_id, direction, feature))
        # Source-inspected routes in the reported gaps: Bardia–Sollum road and
        # Fort Maddelenna–Bir el Sheferzen track. This is evidence connectivity,
        # not movement permission or authority for within-hex junctions.
        for feature, start, destination in (
            ('road', 'C4321', 'C4021'), ('track', 'C3019', 'C3419'),
        ):
            reachable = {row[0] for row in self.db.execute("""
                WITH RECURSIVE reach(hex) AS (
                  VALUES (?) UNION SELECT n.to_hex FROM network_links n JOIN reach r ON n.from_hex=r.hex
                  WHERE n.feature=? AND n.status='provisional'
                ) SELECT hex FROM reach
            """, (start, feature))}
            self.assertIn(destination, reachable)
        self.assertEqual(self.db.execute("""
            SELECT n.id FROM network_links n JOIN hexes a ON a.id=n.from_hex JOIN hexes b ON b.id=n.to_hex
            WHERE a.section<>b.section
        """).fetchall(), [])

    def test_regeneration_is_logically_identical_and_changes_are_detectable(self):
        with sqlite3.connect(":memory:") as second:
            module.populate(second, self.model)
            self.assertEqual(module.logical_dump(self.db), module.logical_dump(second))
            second.execute("UPDATE hexes SET center_x=center_x+1 WHERE id='A4829'")
            self.assertNotEqual(module.logical_dump(self.db), module.logical_dump(second))


if __name__ == "__main__":
    unittest.main()
