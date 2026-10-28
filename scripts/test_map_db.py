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

    def test_unfinished_depictions_preserve_observed_crossings_without_network_links(self):
        # Independently traced source symbols, including short bends which an
        # endpoint-only reachability assertion would miss. These are not routes
        # available to movement: the join only recovers each depicted shared side.
        for survey, depiction, route in (
            ('sollum-buq-buq-unfinished-road', 'unfinished_road',
             'C4021 C3922 C3822 C3823 C3824 C3825 C3926 C3927 C4027 C4028 C4029 C4130'),
            ('capuzzo-sofafi-unfinished-railroad', 'unfinished_railroad',
             'C4217 C4218 C4119 C4019 C4020 C3921 C3821 C3722 C3622 C3523 C3423 '
             'C3324 C3325 C3326 C3226 C3227 C3328 C3329 C3429 C3430 C3531 C3631 C3632 C3633'),
        ):
            cells = route.split()
            self.assertEqual(self.db.execute(
                'SELECT count(*) FROM observations WHERE survey=? AND depiction=? AND status=\'unresolved\' AND value IS NULL',
                (survey, depiction)).fetchone()[0], len(cells) - 1)
            self.assertEqual(self.db.execute(
                'SELECT count(*) FROM network_links WHERE survey=?', (survey,)).fetchone()[0], 0)
            for start, end in zip(cells, cells[1:]):
                with self.subTest(start=start, end=end, depiction=depiction):
                    self.assertIsNotNone(self.db.execute('''
                        SELECT 1 FROM observations o JOIN hexsides s USING(hex,direction)
                        WHERE o.survey=? AND ((s.hex=? AND s.other=?) OR (s.hex=? AND s.other=?))
                    ''', (survey, start, end, end, start)).fetchone())
        self.assertEqual(self.db.execute("""
            SELECT DISTINCT depiction FROM observations WHERE survey='matruh-rail-west'
        """).fetchall(), [('unfinished_railroad',)])
        for update in (
            "UPDATE observations SET status='provisional',value='railroad',source_label=NULL WHERE depiction IS NOT NULL",
            "UPDATE observations SET depiction='unfinished_road' WHERE kind='terrain'",
            "UPDATE observations SET depiction='finished_road' WHERE depiction IS NOT NULL",
        ):
            with self.assertRaises(sqlite3.IntegrityError):
                self.db.execute(update)
        self.assertEqual(self.db.execute("""
            SELECT depiction FROM observations WHERE hex='C3019' AND direction='east' AND depiction IS NOT NULL
        """).fetchall(), [])

    def test_siwa_matruh_unfinished_road_is_depicted_but_not_an_operational_road(self):
        # Read from source (7800,3200,1150,1380) and (8550,1800,1200,1500).
        # C1833 and D1800 overlap on paper; do not invent a section-seam link.
        for route in (
            'C0127 C0227 C0327 C0426 C0527 C0627 C0728 C0828 C0929 C1029 '
            'C1130 C1230 C1331 C1332 C1432 C1533 C1633 C1733 C1833',
            'D1800 D1901 D2001 D2002 D2103 D2203 D2204 D2305 D2405 D2406 '
            'D2507 D2607 D2708 D2808 D2909 D3009 D3110 D3210 D3311 D3411 D3512 D3612',
        ):
            cells = route.split()
            for start, end in zip(cells, cells[1:]):
                with self.subTest(start=start, end=end):
                    self.assertIsNotNone(self.db.execute('''
                        SELECT 1 FROM observations o JOIN hexsides s USING(hex,direction)
                        WHERE o.depiction='unfinished_road' AND o.status='unresolved' AND o.value IS NULL
                          AND ((s.hex=? AND s.other=?) OR (s.hex=? AND s.other=?))
                    ''', (start, end, end, start)).fetchone())
                    self.assertIsNone(self.db.execute('''
                        SELECT 1 FROM network_links WHERE from_hex=? AND to_hex=? AND feature='road'
                    ''', (start, end)).fetchone())

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
            ('C3226', 'north_west', 'track', False),  # Disconnected cross-ties, not dashes.
            ('D2224', 'north_east', 'track', False),
            ('C3922', 'north_west', 'track', False),  # Unfinished double road capsules.
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
        # Check each source crossing, not reachability through an alternate path.
        # The first seven traces cover gray dashed routes in the Maddelenna / Bir
        # el'esc Scegga crop (6950,2010,770,760); the rest cover C3008 and Sidi Azeiz.
        # A grid-overprinted dash is still a track; blue barred sides are not
        # evidence for joining the two independent north/south routes.
        for route in (
            'C3015 C3016 C3017 C3018 C3019',
            'C3020 C3021 C3022 C3023 C3024',
            'C3018 C2918 C2817 C2717 C2616 C2516',
            'C3518 C3418 C3318 C3218 C3119 C3019 C2919 C2819 C2720 C2619 C2519',
            'C3519 C3419 C3319 C3219 C3120 C3020 C2921 C2820 C2721 C2621 C2522',
            'C3516 C3416 C3417 C3418 C3419 C3420 C3321 C3322 C3222 C3223 C3224',
            'C3319 C3419 C3520',
            'C2908 C3008 C3009',
            'C4216 C4217 C4218 C4219 C4220',
            'C4017 C4118 C4218 C4319',
        ):
            cells = route.split()
            for start, destination in zip(cells, cells[1:]):
                with self.subTest(start=start, destination=destination):
                    self.assertIsNotNone(self.db.execute("""
                        SELECT 1 FROM network_links WHERE from_hex=? AND to_hex=?
                        AND feature='track' AND status='provisional'
                    """, (start, destination)).fetchone())
        self.assertEqual(self.db.execute("""
            SELECT feature FROM network_links WHERE from_hex='C3019' AND to_hex='C3020'
        """).fetchall(), [])
        self.assertEqual(self.db.execute("""
            SELECT n.id FROM network_links n JOIN hexes a ON a.id=n.from_hex JOIN hexes b ON b.id=n.to_hex
            WHERE a.section<>b.section
        """).fetchall(), [])

    def test_reported_source_gaps_and_neighboring_track_crossings(self):
        # Independently read from the cited source crops, not computed from
        # existing links. Include each side so an alternate path cannot hide gaps.
        for route in (
            'C3111 C3011 C3012 C3013 C3014 C3015 C3016',
            'C3008 C3009 C3010 C3111 C3211 C3312 C3412',
            'C3409 C3310 C3210 C3111 C3010 C2911 C2811 C2712 C2612',
            'C2619 C2519 C2418 C2318 C2218 C2217 C2117 C2116 C2016 C1916 C1815 C1715',
            'C1619 C1519 C1418 C1319 C1218 C1119 C1019 C0920 C0820 C0821',
            'C1119 C1118 C1117 C1116 C1115',
            'C2410 C2411',
            'C2414 C2415',
            'C1415 C1515',
            'C0925 C0825',
        ):
            cells = route.split()
            for start, end in zip(cells, cells[1:]):
                with self.subTest(start=start, end=end):
                    self.assertIsNotNone(self.db.execute('''
                        SELECT 1 FROM network_links WHERE from_hex=? AND to_hex=?
                          AND feature='track' AND status='provisional'
                    ''', (start, end)).fetchone())
        # Enlarged source crops against calibrated edges rule out these detours:
        # El Cuasc goes straight into C3011; the northern bend uses C2318.
        # C0925 bends south into C0825, not east into C0926.
        for start, end in (('C3111', 'C3112'), ('C3011', 'C3112'),
                           ('C2418', 'C2319'), ('C2319', 'C2218'),
                           ('C0925', 'C0926')):
            with self.subTest(unsupported_crossing=(start, end)):
                self.assertIsNone(self.db.execute('''
                    SELECT 1 FROM network_links WHERE from_hex=? AND to_hex=? AND feature='track'
                ''', (start, end)).fetchone())

    def test_source_points_retain_positions_and_do_not_promote_categories(self):
        self.assertEqual(self.db.execute('''
            SELECT symbol,center_x,center_y,label,status FROM source_points WHERE hex='C3111'
        ''').fetchall(), [('village_bir_dot', 6567.0, 2324.1, 'El Cuasc', 'unresolved')])
        self.assertEqual(dict(self.db.execute('SELECT symbol,count(*) FROM source_points GROUP BY symbol')),
                         {'village_bir_dot': 117, 'oasis': 2, 'port_icon': 2})
        for hex_id, examination in self.model['examinations'].items():
            for point in examination['points']:
                self.assertIsNotNone(self.db.execute('''
                    SELECT 1 FROM source_points WHERE hex=? AND symbol=? AND center_x=? AND center_y=?
                      AND label IS ? AND status=? AND source_refs=?
                ''', (hex_id, point['symbol'], *point['center'], point.get('label'), point['status'],
                      module.encode(point['source_refs']))).fetchone())
        self.assertEqual(self.db.execute('''
            SELECT status,value FROM observations WHERE survey='label-C3111'
        ''').fetchone(), ('unresolved', None))

    def test_regeneration_is_logically_identical_and_changes_are_detectable(self):
        with sqlite3.connect(":memory:") as second:
            module.populate(second, self.model)
            self.assertEqual(module.logical_dump(self.db), module.logical_dump(second))
            second.execute("UPDATE hexes SET center_x=center_x+1 WHERE id='A4829'")
            self.assertNotEqual(module.logical_dump(self.db), module.logical_dump(second))


if __name__ == "__main__":
    unittest.main()
