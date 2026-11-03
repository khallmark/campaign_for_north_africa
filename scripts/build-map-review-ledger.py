#!/usr/bin/env python3
"""Expand explicit visual-review notes; never infer review from raster detection."""
import argparse
from collections import defaultdict
from datetime import date
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
REVIEWS = ROOT / "data/map/source-reviews.json"
OUTPUT = ROOT / "data/map/source-review-ledger.json"
FAMILIES = {"surface", "terrain", "transport", "hexside", "locations"}


def build_ledger(model, reviews):
    """Return one review record for every current inventory cell, or reject drift."""
    geometry = {k: model[k] for k in ("hexes", "hexsides")}
    geometry_hash = hashlib.sha256(json.dumps(geometry, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
    if reviews["version"] != 1 or reviews["geometry_sha256"] != geometry_hash:
        raise ValueError("review geometry changed; reconcile inspected outlines before updating the pin")
    if reviews["sources"] != model["sources"] or reviews["map_source"] not in model["sources"]:
        raise ValueError("review source pins differ from the model; re-inspect changed sources")
    cells = {}
    for hexagon in model["hexes"]:
        hex_id = hexagon["id"]
        crop = model["examinations"][hex_id]["crop"]
        cells[hex_id] = {
            "hex": hex_id, "section": hexagon["section"],
            "source_ref": reviews["map_source"] + "#xywh=" + ",".join(map(str, crop)),
            "status": "not_reviewed", "human_reviewed": False,
            "findings": {family: {"status": "not_reviewed"} for family in sorted(FAMILIES)},
            "survey_refs": [], "uncertainty": ["Source cell not visually reviewed in this audit."],
        }
    neighbors = {(s["hex"], s["direction"]): s["other"] for s in model["hexsides"]}
    survey_cells = {}
    for survey in model["surveys"]:
        owners = set()
        for record in survey["records"]:
            if "side" in record:
                side = record["side"]
                owners.update((side["hex"], neighbors[(side["hex"], side["direction"])]))
            else:
                owners.add(record["hex"])
        survey_cells[survey["id"]] = owners
    batches = {}
    for batch in reviews["batches"]:
        if batch["id"] in batches or batch["method"] != "agent_visual" or not batch["notes"].strip():
            raise ValueError("expected unique batch ID, agent_visual method and inspection notes")
        date.fromisoformat(batch["date"])
        batches[batch["id"]] = {k: batch[k] for k in ("date", "method", "notes")}
        for group in batch["groups"]:
            findings = {**batch["common_findings"], **group["findings"]}
            if set(findings) != FAMILIES or any(
                f["status"] not in {"observed", "not_observed", "unresolved"} or not f["notes"].strip()
                for f in findings.values()
            ):
                raise ValueError("every reviewed cell needs five explicit family findings; detector status is not review")
            if not group["hexes"] or not group["uncertainty"] or any(not n.strip() for n in group["uncertainty"]):
                raise ValueError("review groups need explicit cells and uncertainty notes")
            for hex_id in group["hexes"]:
                if hex_id not in cells or cells[hex_id]["status"] != "not_reviewed":
                    raise ValueError(f"unknown or multiply reviewed cell: {hex_id}")
                for survey in group["survey_refs"]:
                    if hex_id not in survey_cells.get(survey, set()):
                        raise ValueError(f"survey {survey} does not cover {hex_id}")
                cells[hex_id].update(status="agent_reviewed", batch=batch["id"], findings=findings,
                                     survey_refs=group["survey_refs"], uncertainty=group["uncertainty"])
    coverage = defaultdict(lambda: {"total": 0, "agent_reviewed": 0, "not_reviewed": 0})
    for cell in cells.values():
        coverage[cell["section"]]["total"] += 1
        coverage[cell["section"]][cell["status"]] += 1
    return {
        "version": 1, "baseline_commit": reviews["baseline_commit"], "geometry_sha256": geometry_hash,
        "sources": reviews["sources"], "batches": batches, "coverage": dict(sorted(coverage.items())),
        "cells": list(cells.values()),
    }


def main():
    parser = argparse.ArgumentParser(
        description=__doc__,
        epilog="Examples: python3 scripts/build-map-review-ledger.py --check; python3 scripts/build-map-review-ledger.py --write",
    )
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="check source pins and committed ledger (default; no writes)")
    mode.add_argument("--write", action="store_true", help="regenerate the ledger from explicitly authored review notes")
    args = parser.parse_args()
    try:
        reviews = json.loads(REVIEWS.read_text())
        for path, expected in reviews["sources"].items():
            if hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != expected:
                raise ValueError(f"source fingerprint changed: {path}; re-inspect before regeneration")
        model = json.loads(subprocess.check_output(
            ["cargo", "run", "--quiet", "--locked", "-p", "cna-map", "--bin", "map-model"], cwd=ROOT,
        ))
        ledger = build_ledger(model, reviews)
        # One cell per line keeps coverage changes reviewable without raster pixels.
        header = {k: v for k, v in ledger.items() if k != "cells"}
        output = json.dumps(header, indent=2, ensure_ascii=False)[:-2] + ',\n  "cells": [\n'
        output += ",\n".join("    " + json.dumps(c, ensure_ascii=False, separators=(",", ":")) for c in ledger["cells"])
        output += "\n  ]\n}\n"
        if args.write:
            OUTPUT.write_text(output)
        elif not OUTPUT.exists() or OUTPUT.read_text() != output:
            raise ValueError("source-review ledger is stale or missing; inspect changes, then run with --write")
        print(json.dumps(ledger["coverage"], sort_keys=True))
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"error: {error}\n")


if __name__ == "__main__":
    main()
