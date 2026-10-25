#!/usr/bin/env python3
"""Regenerate a private SQLite research model from Rust geometry and pinned surveys."""
import argparse
from datetime import date
import hashlib
import json
from pathlib import Path
import sqlite3
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
DEFAULT = ROOT / "data/map/map.sqlite"
INPUTS = [
    "public/data/map/sources.json", "data/map/surveys.json", "data/map/schema.sql",
    "crates/cna-map/src/geometry.rs", "crates/cna-map/src/calibration.rs",
    "crates/cna-map/src/hex.rs", "crates/cna-map/src/terrain.rs",
    "crates/cna-map/src/model.rs", "crates/cna-map/src/bin/map-model.rs",
    "scripts/import-map-db.py", "Cargo.lock",
]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def encode(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def export_model():
    result = subprocess.run(
        ["cargo", "run", "--quiet", "--locked", "-p", "cna-map", "--bin", "map-model"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    )
    model = json.loads(result.stdout)
    census = json.loads((ROOT / "public/data/map/sources.json").read_text())
    sources = {**model["sources"], census["source"]["path"]: census["source"]["sha256"]}
    if model["sources"].get(census["image"]["path"]) != census["image"]["sha256"]:
        raise ValueError("survey image and geometry census fingerprints disagree")
    for path, expected in sources.items():
        if digest(ROOT / path) != expected:
            raise ValueError(f"source fingerprint changed: {path}; review before regeneration")
    model["sources"] = sources
    return model


def populate(db, model):
    db.executescript((ROOT / "data/map/schema.sql").read_text())
    fingerprints = {path: digest(ROOT / path) for path in INPUTS}
    db.executemany("INSERT INTO metadata VALUES (?,?)", [
        ("generator", "cna-map-model"),
        ("schema_version", "1"), ("membership", model["membership"]),
        ("input_fingerprints", encode(fingerprints)),
        ("model_sha256", hashlib.sha256(encode(model).encode()).hexdigest()),
        ("seams", "unresolved; no cross-section movement or identity equivalence"),
        ("terrain_completeness", "partial; absence of an observation is unknown, never Clear"),
    ])
    db.executemany("INSERT INTO sources VALUES (?,?)", sorted(model["sources"].items()))
    db.executemany("INSERT INTO hexes(id,section,col,row,center_x,center_y,corners) VALUES (?,?,?,?,?,?,?)", [
        (h["id"], h["section"], h["col"], h["row"], *h["center"], encode(h["corners"])) for h in model["hexes"]
    ])
    db.executemany("INSERT INTO hexsides VALUES (?,?,?,?)", [
        (s["hex"], s["direction"], s["other"], encode(s["edge"])) for s in model["hexsides"]
    ])
    for survey in model["surveys"]:
        db.execute("INSERT INTO surveys VALUES (?,?,?)", (survey["id"], survey["label"], survey["notes"]))
        for index, assignment in enumerate(survey["records"]):
            record = assignment["record"]
            observation = record["observation"]
            provenance = record["provenance"]
            # Validate metadata that the Rust vocabulary deliberately leaves as strings.
            date.fromisoformat(provenance["date"])
            identifier = f'{survey["id"]}:{index}'
            location = assignment.get("side", {"hex": assignment.get("hex")})
            value = observation.get("value")
            if isinstance(value, dict):
                value = encode(value)
            db.execute("INSERT INTO observations VALUES (?,?,?,?,?,?,?,?,?)", (
                identifier, survey["id"], assignment["kind"], location["hex"], location.get("direction"),
                observation["status"], value, observation.get("source_label"), encode(provenance),
            ))
            for reference in provenance["source_refs"]:
                db.execute("INSERT INTO observation_sources VALUES (?,?,?)", (identifier, reference.split("#")[0], reference))
    db.execute("UPDATE hexes SET terrain_status='partial' WHERE id IN (SELECT hex FROM observations WHERE kind='terrain')")
    if db.execute("PRAGMA foreign_key_check").fetchall():
        raise ValueError("invalid map foreign keys")
    db.commit()


def logical_dump(db):
    return "\n".join(db.iterdump())


def main():
    parser = argparse.ArgumentParser(description=__doc__, epilog="Examples: python3 scripts/import-map-db.py --write; python3 scripts/import-map-db.py --check")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true", help="atomically regenerate the disposable derived database")
    mode.add_argument("--check", action="store_true", help="read-only source and complete logical database comparison (default)")
    parser.add_argument("--output", type=Path, default=DEFAULT, help="private database path, never under public/ or dist/")
    args = parser.parse_args()
    output = args.output.resolve()
    if output.suffix != ".sqlite" or any(output.is_relative_to(ROOT / folder) for folder in ("public", "dist")):
        parser.error("output must be a private .sqlite path outside public/ and dist/")
    model = export_model()
    with sqlite3.connect(":memory:") as expected:
        populate(expected, model)
        if args.write:
            if output.exists():
                with sqlite3.connect(output.as_uri() + "?mode=ro", uri=True) as previous:
                    if previous.execute("SELECT value FROM metadata WHERE key='generator'").fetchone() != ("cna-map-model",):
                        raise ValueError("refusing to replace a database not owned by cna-map-model")
            output.parent.mkdir(parents=True, exist_ok=True)
            with tempfile.NamedTemporaryFile(dir=output.parent, suffix=".sqlite", delete=False) as temp:
                temporary = Path(temp.name)
            try:
                with sqlite3.connect(temporary) as destination:
                    expected.backup(destination)
                temporary.replace(output)
            finally:
                temporary.unlink(missing_ok=True)
        else:
            with sqlite3.connect(output.as_uri() + "?mode=ro", uri=True) as actual:
                if logical_dump(expected) != logical_dump(actual):
                    raise ValueError("map database stale or modified; review inputs then run --write")
        print(encode({"database": str(output), "check": not args.write, "coverage": expected.execute("SELECT * FROM coverage").fetchall(), "observations": expected.execute("SELECT count(*) FROM observations").fetchone()[0]}))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, sqlite3.Error, subprocess.CalledProcessError) as error:
        raise SystemExit(str(error) + ("\n" + error.stderr if isinstance(error, subprocess.CalledProcessError) else "")) from error
