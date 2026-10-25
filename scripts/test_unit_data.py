"""Run with: uv run --with jsonschema python -m unittest discover -s scripts -p test_unit_data.py."""

import copy
import importlib.util
import subprocess
import sys
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("unit_import", ROOT / "scripts/import-unit-data.py")
importer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(importer)


class UnitDataTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.catalog = importer.read_json(ROOT / "public/data/unit-catalog.json")
        cls.inventory = importer.read_json(ROOT / "public/data/unit-identities.json")
        cls.census = importer.read_json(ROOT / importer.CENSUS)
        cls.evidence = importer.read_json(ROOT / importer.EVIDENCE)
        cls.reference, cls.matrix = importer.build(ROOT)
        cls.records = {r["id"]: r for r in cls.reference["records"] + cls.reference["profiles"]}
        schema = importer.read_json(ROOT / "docs/unit-reference.schema.json")
        Draft202012Validator.check_schema(schema)
        cls.validator = Draft202012Validator(schema)

    def test_schema_accepts_current_data_and_unresolved_cases(self):
        self.validator.validate(self.reference)
        unit = copy.deepcopy(next(u for u in self.reference["units"]
                                  if u["token_link_status"] == "unresolved"
                                  and u["generated_art"]["status"] == "missing"))
        self.assertEqual(unit["counter_gpids"], [])
        self.assertIsNone(unit["generated_art"]["portrait_path"])
        self.assertIsNone(unit["generated_art"]["atlas_path"])
        self.assertIsNone(unit["organization_at_arrival"])
        self.assertIsNone(unit["description"])
        counter = copy.deepcopy(next(c for c in self.reference["counters"]
                                     if c["link_status"] == "unresolved"))
        counter.update(layers=[], basic_image_id=None, image_ids=[])
        self.assertIsNone(counter["canonical_unit_id"])
        for status in ["missing", "candidate"]:
            with self.subTest(status=status):
                unit["generated_art"]["status"] = status
                self.validator.validate({**self.reference, "units": [unit], "counters": [counter]})

    def test_formation_reference_contract_rejects_partial_totals_and_private_fields(self):
        record = self.records['oa-124-002-13th-light-aa-regt']
        for mutation in ('unresolved-number', 'missing-unit-shape', 'private-field'):
            changed = copy.deepcopy(record)
            reference = changed['formation_reference']
            if mutation == 'unresolved-number':
                reference['arrival_totals']['anti_air']['status'] = 'unresolved'
            elif mutation == 'missing-unit-shape':
                reference['components'][0]['toe'] = 6
            else:
                reference['private_text'] = 'not an allowed projection field'
            with self.subTest(mutation=mutation):
                self.assertFalse(self.validator.is_valid({**self.reference, 'records': [changed]}))

        coastal = self.records['oa-155-022-2nd-tobruk-cd-arty-group']
        for path in (('base_stats', 'capability_points'), ('arrival', 'stats', 'capability_points'),
                     ('arrival', 'toe_strength'), ('arrival', 'morale'),
                     ('components', 0, 'toe'), ('components', 0, 'per_toe_stats', 'barrage'),
                     ('arrival_totals', 'barrage')):
            for invalid in ('missing', None, '', 1):
                changed = copy.deepcopy(coastal)
                cell = changed['formation_reference']
                for key in path:
                    cell = cell[key]
                if invalid == 'missing':
                    del cell['unit']
                else:
                    cell['unit'] = invalid
                with self.subTest(path=path, unit=invalid):
                    self.assertFalse(self.validator.is_valid({**self.reference, 'records': [changed]}))

    def test_schema_rejects_ready_art_without_canonical_paths(self):
        ready = next(u for u in self.reference["units"] if u["generated_art"]["status"] == "ready")
        for field in ["portrait_path", "atlas_path"]:
            for invalid in [None, "", "/__source-tokens/original.svg"]:
                with self.subTest(field=field, invalid=invalid):
                    unit = copy.deepcopy(ready)
                    unit["generated_art"][field] = invalid
                    self.assertFalse(self.validator.is_valid({**self.reference, "units": [unit]}))

    def test_schema_rejects_malformed_counter_labels_and_layers(self):
        for field, invalid in [("label", {}), ("layers", 42), ("layers", [42]),
                               ("layers", [{"name": "incomplete"}])]:
            with self.subTest(field=field, invalid=invalid):
                counter = {**self.reference["counters"][0], field: invalid}
                self.assertFalse(self.validator.is_valid({**self.reference, "counters": [counter]}))

    def test_schema_requires_nonnegative_integer_coverage_counts(self):
        for field in self.reference["coverage"]:
            if field == "complete":
                continue
            with self.subTest(field=field):
                coverage = dict(self.reference["coverage"])
                del coverage[field]
                self.assertFalse(self.validator.is_valid({**self.reference, "coverage": coverage}))
                for invalid in [-1, True, "118"]:
                    coverage[field] = invalid
                    self.assertFalse(self.validator.is_valid({**self.reference, "coverage": coverage}))

    def test_every_identity_source_slot_and_artwork_is_preserved(self):
        units = {u["id"]: u for u in self.reference["units"]}
        self.assertEqual(set(units), {i["id"] for i in self.inventory["entries"]})
        self.assertEqual({s["gpid"] for s in self.reference["counters"]},
                         {s["gpid"] for s in self.census["slots"]})
        self.assertEqual(len(self.reference["counters"]), 2468)
        self.assertEqual(self.reference["identity_redirects"], self.inventory["identity_redirects"])
        self.assertEqual(len(self.reference["identity_redirects"]), 18)
        for identity in self.inventory["entries"]:
            unit = units[identity["id"]]
            self.assertEqual(unit["generated_art"], identity["sprite"])
            self.assertEqual(unit["source_record_ids"], identity["source_entry_ids"])
            if identity["sprite"]["status"] == "ready":
                self.assertTrue(unit["counter_gpids"], identity["id"])
        self.assertFalse(self.reference["coverage"]["complete"])

    def test_foreign_keys_and_bidirectional_counter_links(self):
        ref = self.reference
        units = {u["id"]: u for u in ref["units"]}
        counters = {s["gpid"]: s for s in ref["counters"]}
        assets = {a["id"] for a in ref["token_assets"]}
        types = {t["id"] for t in ref["unit_types"]}
        profiles = {p["id"] for p in ref["profiles"]}
        records = {r["id"] for r in ref["records"]}
        sources = {s["id"] for s in ref["sources"]}
        for record in ref["records"] + ref["profiles"]:
            self.assertLessEqual(set(record["source_refs"]), sources)
            for stat in record["stats"].values():
                self.assertLessEqual(set(stat["source_refs"]), sources)
        for unit in units.values():
            self.assertLessEqual(set(unit["source_record_ids"]), records)
            self.assertLessEqual(set(unit["unit_type_ids"]), types)
            self.assertLessEqual(set(unit["characteristic_profile_ids"]), profiles)
            for gpid in unit["counter_gpids"]:
                self.assertEqual(counters[gpid]["canonical_unit_id"], unit["id"])
        for counter in counters.values():
            self.assertLessEqual(set(counter["image_ids"]), assets)
            if counter["canonical_unit_id"]:
                self.assertIn(counter["gpid"], units[counter["canonical_unit_id"]]["counter_gpids"])
        self.assertEqual(len(self.matrix["vassal_slots"]), len(counters))

    def test_names_alone_cannot_link_and_wrong_board_is_rejected(self):
        census = copy.deepcopy(self.census)
        anchor = next(s for s in census["slots"] if s["gpid"] == "3702")
        copy_slot = copy.deepcopy(anchor)
        copy_slot.update(gpid="test-same-name", palette="units", duplicate_palette_candidates=[])
        census["slots"].append(copy_slot)
        links = importer.link_identities(self.inventory, census, self.evidence)
        self.assertNotIn("test-same-name", links)
        self.assertEqual(links["3702"]["unit_id"], "unit-commonwealth-2nd-bays")
        anchor["palette_path"] = ["VASSAL.build.widget.WidgetMap:BR 2nd Armoured Division"]
        with self.assertRaisesRegex(ValueError, "board mismatch"):
            importer.link_identities(self.inventory, census, self.evidence)

    def test_palette_copy_requires_image_evidence_not_just_name(self):
        census = copy.deepcopy(self.census)
        slot = next(s for s in census["slots"] if "3702" in s["duplicate_palette_candidates"])
        slot["basic_image"] = "wrong-unit.svg"
        with self.assertRaisesRegex(ValueError, "Invalid census palette link"):
            importer.link_identities(self.inventory, census, self.evidence)

    def test_errata_are_applied_once_without_erasing_original_observations(self):
        pak = self.records["weapons-ge-7-62cm-pak-r"]["stats"]["capability_points"]
        marder = self.records["weapons-ge-marder-iii-sp"]["stats"]["capability_points"]
        self.assertEqual((pak["value"], marder["value"]), (15, 25))
        self.assertIsNone(pak["original_observation"]["value"])
        self.assertEqual(pak["original_observation"]["status"], "unresolved")
        pz = self.records["weapons-ge-pz-iii-e"]["stats"]
        self.assertEqual((pz["armor_protection"]["value"], pz["close_assault_offensive"]["value"],
                          pz["fuel_rate"]["value"], pz["breakdown_shift"]["value"]), (3, 4, 3, 0))
        self.assertTrue(pz["breakdown_shift"]["conditional"])
        self.assertEqual(pz["vulnerability"]["status"], "not_applicable")
        self.assertIsNone(pz["vulnerability"]["value"])
        self.assertNotIn('original_observation', pak['original_observation'])
        projected = copy.deepcopy(self.catalog)
        projected_pak = next(r for r in projected['entries'] if r['id'] == 'weapons-ge-7-62cm-pak-r')
        for field, bad_value in [('value', 16), ('raw', '16'), ('conditional', True), ('status', 'unresolved')]:
            saved = projected_pak['stats']['capability_points'][field]
            projected_pak['stats']['capability_points'][field] = bad_value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'Projected stat disagrees'):
                importer.apply_evidence(ROOT, projected, self.evidence, {})
            projected_pak['stats']['capability_points'][field] = saved
        evidence = copy.deepcopy(self.evidence)
        evidence["errata"].append(evidence["errata"][0])
        with self.assertRaisesRegex(ValueError, "Repeated erratum"):
            importer.apply_evidence(ROOT, self.catalog, evidence, {})

    def test_cpa_zero_unknown_and_conditional_values_are_distinct(self):
        self.assertEqual(self.records["it-pp"]["stats"]["capability_points"]["value"], 0)
        stat = self.records["cw-l"]["stats"]["capability_points"]
        self.assertEqual(stat["value"], 10)
        self.assertEqual(stat["raw"], "10+")
        self.assertIn("Do not treat base as motorized allowance", stat["note"])
        armor = self.records["ge-j"]["stats"]["armor_protection"]
        self.assertEqual(armor["value"], 1)
        self.assertTrue(armor["conditional"])
        self.assertEqual(len(self.reference["cpa_checks"]), 124)
        for row in self.reference["cpa_checks"]:
            self.assertEqual(row["result"], "numeric_agreement", row["record_id"])
        # A disagreement is retained and must not silently replace the manual value.
        catalog = copy.deepcopy(self.catalog)
        catalog["profiles"][0]["stats"]["capability_points"]["value"] = 29
        _, profiles, checks = importer.apply_evidence(ROOT, catalog, self.evidence, {})
        self.assertEqual(checks[0]["result"], "conflict")
        self.assertEqual(profiles[0]["stats"]["capability_points"]["value"], 29)

    def test_original_source_images_are_not_generated_art_or_public_assets(self):
        missing = {a["id"] for a in self.reference["token_assets"] if not a["exists"]}
        self.assertEqual(missing, set(self.census["summary"]["missing_images"]))
        self.assertEqual(len(missing), 14)
        for asset in self.reference["token_assets"]:
            self.assertIsNone(asset["public_path"])
            if asset["exists"]:
                self.assertTrue(asset["private_review_path"].startswith("/__source-tokens/"))
                self.assertEqual(importer.source_info(ROOT, asset["source_path"], {})["sha256"], asset["sha256"])
            else:
                self.assertIsNone(asset["private_review_path"])

    def test_toe_not_vehicle_counts_and_formation_history_not_unit_biography(self):
        units = {u["id"]: u for u in self.reference["units"]}
        bays = units["unit-commonwealth-2nd-bays"]
        self.assertEqual(bays["organization_at_arrival"]["equipment_toe_raw"], "8 × Crusader Mk I, 2 × A13")
        self.assertEqual(bays["organization_at_arrival"]["basic_morale_raw"], "+2 (+1)")
        self.assertEqual(bays["description"]["scope"], "formation_history_not_individual_unit")
        self.assertNotIn("strength", bays)
        self.assertNotIn("current_capability_points", bays)
        self.assertEqual(units["unit-commonwealth-1st-rifle-brigade"]["organization_at_arrival"]["equipment_toe_raw"], "N")

    def test_cli_check_is_read_only_and_generated_files_match(self):
        paths = [ROOT / importer.REFERENCE, ROOT / importer.MATRIX]
        before = [p.read_bytes() for p in paths]
        result = subprocess.run([sys.executable, str(ROOT / "scripts/import-unit-data.py"), "--check"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(before, [p.read_bytes() for p in paths])


if __name__ == "__main__":
    unittest.main()
