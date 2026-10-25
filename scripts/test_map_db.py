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
        self.assertEqual(self.db.execute("SELECT count(*) FROM observation_sources os JOIN sources s ON os.source=s.path WHERE length(s.sha256)=64").fetchone()[0], 204)
        with self.assertRaises(sqlite3.IntegrityError):
            self.db.execute("UPDATE observations SET source_label=NULL WHERE survey='matruh-port-location'")
        for feature in ("road", "railroad"):
            reachable = {row[0] for row in self.db.execute("""
                WITH RECURSIVE reach(hex) AS (
                  VALUES ('D3714') UNION SELECT n.to_hex FROM network_links n JOIN reach r ON n.from_hex=r.hex
                  WHERE n.feature=? AND n.status='provisional'
                ) SELECT hex FROM reach
            """, (feature,))}
            self.assertIn("D3519", reachable)
            self.assertNotIn("E3519", reachable)
            self.assertEqual("D3616" in reachable, feature == "road")
            self.assertEqual("D3516" in reachable, feature == "railroad")
        self.assertEqual(self.db.execute("SELECT count(*) FROM network_links WHERE status='reviewed'").fetchone()[0], 0)

    def test_regeneration_is_logically_identical_and_changes_are_detectable(self):
        with sqlite3.connect(":memory:") as second:
            module.populate(second, self.model)
            self.assertEqual(module.logical_dump(self.db), module.logical_dump(second))
            second.execute("UPDATE hexes SET center_x=center_x+1 WHERE id='A4829'")
            self.assertNotEqual(module.logical_dump(self.db), module.logical_dump(second))


if __name__ == "__main__":
    unittest.main()
