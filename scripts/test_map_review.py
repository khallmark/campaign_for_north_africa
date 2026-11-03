"""Source-review coverage must never be inferred from detector completion."""
import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("map_review", ROOT / "scripts/build-map-review-ledger.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class MapReviewTests(unittest.TestCase):
    def setUp(self):
        self.model = {
            "hexes": [{"id": "A0101", "section": "A"}, {"id": "A0102", "section": "A"}],
            "hexsides": [], "sources": {"map.png": "a" * 64},
            "examinations": {h: {"crop": crop, "families": {"terrain": "detected"}}
                             for h, crop in [("A0101", [1, 2, 20, 30]), ("A0102", [21, 2, 20, 30])]},
            "surveys": [{"id": "existing", "records": [{"hex": "A0102"}]}],
        }
        geometry = {k: self.model[k] for k in ("hexes", "hexsides")}
        self.reviews = {
            "version": 1, "baseline_commit": "b" * 40,
            "geometry_sha256": hashlib.sha256(json.dumps(geometry, sort_keys=True, separators=(",", ":")).encode()).hexdigest(),
            "sources": self.model["sources"], "map_source": "map.png",
            "batches": [{
                "id": "first", "date": "2026-11-03", "method": "agent_visual",
                "notes": "Full outlined source crop inspected, not just detector candidates.",
                "common_findings": {family: {"status": "not_observed", "notes": "No corresponding glyph observed."}
                                    for family in ("surface", "terrain", "transport", "hexside", "locations")},
                "groups": [{"hexes": ["A0101"], "findings": {}, "survey_refs": [],
                            "uncertainty": ["Agent interpretation; human review pending."]}],
            }],
        }

    def test_unread_cells_remain_unreviewed_despite_detector_and_survey_evidence(self):
        ledger = module.build_ledger(self.model, self.reviews)
        cells = {c["hex"]: c for c in ledger["cells"]}
        self.assertEqual(set(cells), {"A0101", "A0102"})
        self.assertEqual(cells["A0101"]["status"], "agent_reviewed")
        self.assertEqual(cells["A0101"]["source_ref"], "map.png#xywh=1,2,20,30")
        self.assertEqual(cells["A0102"]["status"], "not_reviewed")
        self.assertEqual({f["status"] for f in cells["A0102"]["findings"].values()}, {"not_reviewed"})
        self.assertEqual(cells["A0102"]["survey_refs"], [])
        self.assertTrue(all(c["human_reviewed"] is False for c in cells.values()))
        self.assertEqual(ledger["coverage"]["A"], {"total": 2, "agent_reviewed": 1, "not_reviewed": 1})

    def test_rejects_overclaimed_coverage_and_stale_evidence(self):
        for fault in ("duplicate", "unknown", "missing_family", "invalid_status", "wrong_survey",
                      "unknown_survey", "geometry", "source", "human", "empty_uncertainty"):
            with self.subTest(fault=fault):
                reviews = copy.deepcopy(self.reviews)
                batch = reviews["batches"][0]
                group = batch["groups"][0]
                if fault == "duplicate":
                    group["hexes"].append("A0101")
                elif fault == "unknown":
                    group["hexes"] = ["A0100"]
                elif fault == "missing_family":
                    del batch["common_findings"]["transport"]
                elif fault == "invalid_status":
                    group["findings"]["terrain"] = {"status": "complete", "notes": "Detector complete"}
                elif fault == "wrong_survey":
                    group["survey_refs"] = ["existing"]
                elif fault == "unknown_survey":
                    group["survey_refs"] = ["missing"]
                elif fault == "geometry":
                    reviews["geometry_sha256"] = "0" * 64
                elif fault == "source":
                    reviews["sources"]["map.png"] = "0" * 64
                elif fault == "human":
                    batch["method"] = "human"
                else:
                    group["uncertainty"] = []
                with self.assertRaises(ValueError):
                    module.build_ledger(self.model, reviews)

    def test_committed_ledger_matches_review_notes_and_current_inventory(self):
        model = json.loads(subprocess.check_output(
            ["cargo", "run", "--quiet", "--locked", "-p", "cna-map", "--bin", "map-model"], cwd=ROOT,
        ))
        reviews = json.loads((ROOT / "data/map/source-reviews.json").read_text())
        committed = json.loads((ROOT / "data/map/source-review-ledger.json").read_text())
        self.assertEqual(committed, module.build_ledger(model, reviews))


if __name__ == "__main__":
    unittest.main()
