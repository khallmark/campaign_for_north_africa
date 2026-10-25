#!/usr/bin/env python3
"""Import VASSAL into the committed private SQLite catalog, then apply SQL patches.

Examples (from any directory):
  python3 scripts/import-catalog-db.py              # refresh source tables + new SQL patches
  python3 scripts/import-catalog-db.py --check      # source-dependent, read-only drift check
  python3 scripts/import-catalog-db.py --patch-only # apply new SQL using committed source tables
  python3 scripts/import-catalog-db.py --bootstrap  # ONE-TIME legacy JSON migration; refuses overwrite

Python 3.11+, SQLite 3.37+ (STRICT tables) and Cargo are required for source import.
--patch-only requires only Python/SQLite. Images and saves are never embedded.
The database is private reference data, NOT a public browser bundle or game state.
"""

import argparse
from contextlib import closing
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
import subprocess
import sys
import tempfile
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[1]
DATABASE = ROOT / "data/catalog.sqlite"
SQL = ROOT / "data/sql"
MODULE = Path("docs/CNAv2.1.0")


def json_text(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True)


def read_json(path):
    return json.loads(path.read_text())


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def connect(path, readonly=False):
    db = sqlite3.connect(path.resolve().as_uri() + ("?mode=ro" if readonly else ""), uri=True)
    db.execute("PRAGMA foreign_keys=ON")
    return db


def insert(db, table, values):
    db.execute(f'INSERT INTO "{table}" VALUES ({",".join("?" for _ in values)})', values)


def initialize(db, schema):
    db.executescript(schema.read_text())
    insert(db, "metadata", ("schema_sha256", digest(schema)))
    insert(db, "metadata", ("distribution", "private_reference_only_not_runtime_bundle"))
    db.commit()


def logical_digest(db):
    """Compare schema and rows, not SQLite-version-specific page packing."""
    result = hashlib.sha256()
    schema = db.execute("SELECT type,name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name").fetchall()
    result.update(json_text(schema).encode())
    for kind, name, _ in schema:
        if kind == "table":
            columns = len(db.execute(f'SELECT * FROM "{name}" LIMIT 0').description)
            order = ",".join(str(i + 1) for i in range(columns))
            for row in db.execute(f'SELECT * FROM "{name}" ORDER BY {order}'):
                result.update(json_text([name, row]).encode())
    return result.hexdigest()


def validate(db):
    require(db.execute("PRAGMA user_version").fetchone()[0] == 1, "Unsupported catalog database version")
    require(db.execute("PRAGMA application_id").fetchone()[0] == 1129202001, "Not a CNA catalog database")
    require(db.execute("PRAGMA integrity_check").fetchone()[0] == "ok", "SQLite integrity check failed")
    require(not db.execute("PRAGMA foreign_key_check").fetchall(), "Catalog foreign-key violation")
    require(not db.execute("SELECT 1 FROM identity_redirect r JOIN canonical_unit u ON u.id=r.retired_id").fetchone(),
            "Retired ID is also a live canonical ID")
    # Redirect targets are direct live IDs by FK: no redirect cycles or missing targets.
    for table in ("stat_observation", "stat_correction"):
        require(not db.execute(f"SELECT 1 FROM {table} WHERE status='sourced' AND value IS NULL").fetchone(),
                f"Missing sourced value in {table}")


def apply_patches(db, directory):
    files = sorted(directory.glob("*.sql"))
    names = [p.name for p in files]
    applied = dict(db.execute("SELECT name,sha256 FROM sql_patch ORDER BY name"))
    require(list(applied) == names[:len(applied)], "SQL patch history is not an unchanged prefix; restore old patches and append a new one")
    # Source writes would disappear on the next import. Corrections belong in overlays.
    protected = {"vassal_counter", "token_asset", "counter_image", "counter_layer", "counter_level",
                 "palette_candidate", "stat_observation", "source_document", "source_file", "metadata", "sql_patch"}

    def authorize(action, table, column, database, trigger):
        if action in (sqlite3.SQLITE_INSERT, sqlite3.SQLITE_UPDATE, sqlite3.SQLITE_DELETE) and table in protected:
            return sqlite3.SQLITE_DENY
        return sqlite3.SQLITE_OK

    for path in files:
        checksum = digest(path)
        if path.name in applied:
            require(applied[path.name] == checksum, f"Applied SQL patch changed: {path.name}; add a new numbered patch")
            continue
        # All work is on a disposable candidate DB. Even a script error cannot alter the committed file.
        db.set_authorizer(authorize)
        try:
            db.executescript(path.read_text())
        finally:
            db.set_authorizer(None)
        insert(db, "sql_patch", (path.name, checksum))
        db.commit()
        validate(db)


def import_vassal(db, census, assets):
    require(census["schema_version"] == 1, "Unsupported VASSAL census schema")
    require(len(census["slots"]) == census["summary"]["slots"], "VASSAL slot count mismatch")
    # Deferred FKs preserve reviewed links while source rows are atomically replaced.
    db.execute("BEGIN")
    for table in ("counter_level", "counter_layer", "counter_image", "palette_candidate", "vassal_counter", "token_asset"):
        db.execute(f"DELETE FROM {table}")
    for name, a in sorted(assets.items()):
        insert(db, "token_asset", (name, a["source_path"], int(a["exists"]), a["sha256"], a["size"], None,
                                  "/__source-tokens/" + quote(name, safe="") if a["exists"] else None))
    for position, slot in enumerate(census["slots"]):
        gpid = slot["gpid"]
        insert(db, "vassal_counter", (gpid, position, slot["entry_name"], slot["slot_kind"], slot["side"],
                                     slot["nationality_prefix"], slot["palette"], slot["source_line"], slot["printed_name"],
                                     slot["basic_image"], json_text(slot["palette_path"]), json_text(slot["prototype_refs"])))
        for index, image in enumerate(slot["images"]):
            require(assets[image["name"]]["exists"] == image["exists"], f"Image existence drift: {image['name']}")
            insert(db, "counter_image", (gpid, index, image["name"], image["origin"], image["trait_kind"]))
        for index, layer in enumerate(slot["layers"]):
            insert(db, "counter_layer", (gpid, index, layer["origin"], layer["name"], layer["description"],
                                         layer["initial_state"], layer["first_level_value"]))
            for level_index, level in enumerate(layer["levels"]):
                insert(db, "counter_level", (gpid, index, level_index, level["index"], level["image"], level["label"]))
        for target in slot["duplicate_palette_candidates"]:
            insert(db, "palette_candidate", (gpid, target))
    db.commit()  # Missing GPIDs referenced by reviewed links fail rather than deleting identities.


def verify_sources(root):
    locks = read_json(root / "docs/sources.lock.json")["files"]
    for relative, expected in locks.items():
        path = root / relative
        require(path.resolve().is_relative_to(root.resolve()) and not path.is_symlink(), f"Unsafe source path: {relative}")
        require(path.stat().st_size == expected["size"] and digest(path) == expected["sha256"],
                f"Locked source changed or unhydrated: {relative}")
    return locks


def scan_vassal(root, temp):
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo/bin/cargo")
    output = temp / "vassal-census.json"
    subprocess.run([cargo, "run", "--locked", "-q", "-p", "cna-catalog", "--bin", "vassal-census", "--",
                    "--module", str(root / MODULE / "buildFile.xml"), "--output", str(output)],
                   cwd=root, check=True, stdout=sys.stderr)
    census = read_json(output)
    assets = {}
    for slot in census["slots"]:
        for image in slot["images"]:
            name = image["name"]
            require(Path(name).name == name and "\\" not in name, f"Unsafe source image: {name}")
            path = root / MODULE / "images" / name
            present = path.is_file()
            assets[name] = {"source_path": str(MODULE / "images" / name), "exists": present,
                            "sha256": digest(path) if present else None, "size": path.stat().st_size if present else None}
    return census, assets


def bootstrap_curated(db, root):
    """One-time preservation of reviewed pre-SQLite work, never used by normal imports."""
    catalog = read_json(root / "public/data/unit-catalog.json")
    inventory = read_json(root / "public/data/unit-identities.json")
    reference = read_json(root / "public/data/unit-reference.json")
    matrix = read_json(root / "data/sources/identity-matrix.json")
    texts = read_json(root / "data/sources/vassal-2.1-document-text.json")
    for relative in ("public/data/unit-catalog.json", "public/data/unit-identities.json", "public/data/unit-reference.json",
                     "data/sources/identity-matrix.json", "data/sources/vassal-2.1-document-text.json"):
        insert(db, "metadata", ("bootstrap_sha256:" + relative, digest(root / relative)))
    for key, value in catalog.items():
        if key not in {"entries", "profiles", "sources", "unit_types", "stat_definitions"}:
            insert(db, "metadata", ("legacy_catalog:" + key, json_text(value)))
    for key, value in inventory.items():
        if key not in {"entries", "identity_redirects", "source_row_map"}:
            insert(db, "metadata", ("legacy_inventory:" + key, json_text(value)))
    for source in catalog["source_files"]:
        path = root / source["path"]
        require(digest(path) == source["sha256"], f"Legacy manual source changed: {source['path']}")
        db.execute("INSERT OR IGNORE INTO source_file VALUES (?,?,?)", (source["path"], source["sha256"], path.stat().st_size))
    for source in catalog["sources"]:
        details = {k: v for k, v in source.items() if k not in {"id", "title", "path"}}
        insert(db, "source_citation", (source["id"], source["title"], source["path"], json_text(details)))
    for position, kind in enumerate(catalog["unit_types"]):
        insert(db, "unit_type", (kind["id"], position, kind["name"], json_text({k: v for k, v in kind.items() if k not in {"id", "name"}})))
    for position, stat in enumerate(catalog["stat_definitions"]):
        insert(db, "stat_definition", (stat["id"], position, stat["name"], stat["unit"], stat["description"], json_text(stat["source_refs"])))
    record_columns = {"id", "name", "kind", "side", "nationality", "unit_type", "profile_id", "verification", "note", "stats", "source_refs"}
    for collection in ("entries", "profiles"):
        for position, record in enumerate(catalog[collection]):
            details = {k: v for k, v in record.items() if k not in record_columns}
            # Preserve missing kind/profile_id versus explicit null for compatibility exports.
            details["_absent"] = sorted(record_columns - record.keys())
            insert(db, "catalog_record", (record["id"], collection, position, record["name"], record.get("kind", "profile"),
                                           record["side"], record["nationality"], record["unit_type"], record.get("profile_id"),
                                           record["verification"], record.get("note", ""), json_text(details)))
            for index, source in enumerate(record["source_refs"]):
                insert(db, "record_source", (record["id"], index, source))
            for stat_id, stat in record["stats"].items():
                insert(db, "stat_observation", (record["id"], stat_id, stat["value"], stat["status"], stat["raw"],
                                               int(stat["conditional"]), stat["note"], json_text(stat["source_refs"])))
    unit_columns = {"id", "name", "side", "nationality", "role_id", "entity_kind", "identity_status", "formation_context", "source_entry_ids", "source_refs", "sprite"}
    for position, unit in enumerate(inventory["entries"]):
        details = {k: v for k, v in unit.items() if k not in unit_columns}
        details["_absent"] = sorted(unit_columns - unit.keys())
        insert(db, "canonical_unit", (unit["id"], position, unit["name"], unit["side"], unit["nationality"], unit["role_id"],
                                      unit["entity_kind"], unit["identity_status"], unit.get("formation_context"), json_text(details)))
        for index, record in enumerate(unit["source_entry_ids"]):
            insert(db, "unit_record", (unit["id"], index, record))
        for index, source in enumerate(unit["source_refs"]):
            insert(db, "unit_source", (unit["id"], index, source))
        art = unit["sprite"]
        insert(db, "generated_art", (unit["id"], art["status"], art["portrait_path"], art["atlas_path"],
                                     json_text({k: v for k, v in art.items() if k not in {"status", "portrait_path", "atlas_path"}})))
    for retired, target in inventory["identity_redirects"].items():
        insert(db, "identity_redirect", (retired, target))
    for unit in reference["units"]:
        for kind in ("organization_at_arrival", "description"):
            if unit[kind] is not None:
                insert(db, "unit_context", (unit["id"], kind, json_text(unit[kind])))
    for slot in matrix["vassal_slots"]:
        if slot["unit_id"] is not None:
            insert(db, "unit_counter", (slot["gpid"], slot["unit_id"], slot["status"], json_text(slot["evidence"])))
    for check in reference["cpa_checks"]:
        insert(db, "cpa_check", (check["record_id"], check["result"], json_text(check)))
    for doc in texts["documents"]:
        insert(db, "source_document", (doc["source_path"], doc["kind"], json_text(doc["titles"]), doc["tool"], doc["status"], doc["text"], doc["note"]))
    db.commit()


def update_database(destination, schema, patches, prepare, *, bootstrap=False, check=False, verify=None):
    """Validate a candidate, then atomically replace only if logical content changed."""
    require(not any(destination.resolve().is_relative_to((ROOT / directory).resolve()) for directory in ("public", "dist")),
            "The private database must stay outside public/ and dist/; use a separately reviewed consumer projection")
    require(not destination.is_symlink(), "Database destination must not be a symlink")
    require(not bootstrap or not destination.exists(), "--bootstrap refuses to overwrite an existing database")
    require(bootstrap or destination.is_file(), "Committed database missing; restore it from Git (or use --bootstrap for the one-time migration)")
    require(not any(Path(str(destination) + suffix).exists() for suffix in ("-wal", "-shm", "-journal")),
            "Close SQLite writers and checkpoint before importing")
    original_hash = digest(destination) if destination.exists() else None
    with tempfile.TemporaryDirectory(prefix=".catalog-import-", dir=destination.parent) as directory:
        candidate = Path(directory) / "catalog.sqlite"
        with closing(connect(candidate)) as db:
            if bootstrap:
                initialize(db, schema)
                before = None
            else:
                with closing(connect(destination, readonly=True)) as source:
                    source.backup(db)
                validate(db)
                require(dict(db.execute("SELECT key,value FROM metadata")).get("schema_sha256") == digest(schema),
                        "Schema changed; add a versioned migration instead of editing the initial schema")
                before = logical_digest(db)
            prepare(db, Path(directory))
            apply_patches(db, patches)
            validate(db)
            if verify is not None:
                verify(db)
            after = logical_digest(db)
            db.commit()
        # No timestamp, no changed pages on rerun. --check never replaces the database.
        require(original_hash == (digest(destination) if destination.exists() else None), "Database changed during import; retry after coordinating writers")
        if check:
            require(before == after, "Database drift: run python3 scripts/import-catalog-db.py and review the change")
        elif before != after:
            os.replace(candidate, destination)
        return before != after


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--database", type=Path, default=DATABASE, help="Private database path (default data/catalog.sqlite)")
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--bootstrap", action="store_true", help="One-time transfer of reviewed JSON into a NEW database")
    modes.add_argument("--check", action="store_true", help="Verify source import + patches cause no logical change; never write")
    modes.add_argument("--patch-only", action="store_true", help="Apply new SQL patches without source/Cargo prerequisites")
    args = parser.parse_args()
    try:
        require(sqlite3.sqlite_version_info >= (3, 37), "SQLite 3.37+ required")
        locks = verify_sources(ROOT) if not args.patch_only else None

        def prepare(db, temp):
            if locks is None:
                return
            census, assets = scan_vassal(ROOT, temp)
            import_vassal(db, census, assets)
            for path, source in sorted(locks.items()):
                db.execute("INSERT INTO source_file VALUES (?,?,?) ON CONFLICT(path) DO UPDATE SET sha256=excluded.sha256,size=excluded.size",
                           (path, source["sha256"], source["size"]))
            db.commit()
            if args.bootstrap:
                bootstrap_curated(db, ROOT)

        def verify(db):
            if locks is not None:
                for path, quote_text in db.execute("SELECT source_path,quote FROM correction_evidence"):
                    require(quote_text in (ROOT / path).read_text(), f"Correction quote not found: {path}")

        changed = update_database(args.database, SQL / "schema.sql", SQL / "patches", prepare,
                                  bootstrap=args.bootstrap, check=args.check, verify=verify)
        with closing(connect(args.database, readonly=True)) as db:
            cursor = db.execute("SELECT * FROM coverage")
            coverage = dict(zip([d[0] for d in cursor.description], cursor.fetchone()))
            print(json.dumps({"database": str(args.database), "changed": changed, "coverage": coverage}, indent=2))
    except (OSError, ValueError, KeyError, sqlite3.Error, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Catalog import failed: {error}\n")


if __name__ == "__main__":
    main()
