#!/usr/bin/env python3
"""Import VASSAL into the committed private SQLite catalog, then apply SQL patches.

Examples (from any directory):
  python3 scripts/import-catalog-db.py              # refresh source tables + new SQL patches
  python3 scripts/import-catalog-db.py --check      # source-dependent, read-only drift check
  python3 scripts/import-catalog-db.py --patch-only # apply new SQL using committed source tables
  python3 scripts/import-catalog-db.py --bootstrap  # ONE-TIME legacy JSON migration; refuses overwrite
  python3 scripts/import-catalog-db.py --project-types # project only classifications into existing JSON
  python3 scripts/import-catalog-db.py --project-types --check # read-only projection parity check
  python3 scripts/import-catalog-db.py --project-profiles # reviewed links/citation notes, never numeric stats
  python3 scripts/import-catalog-db.py --project-stats # reviewed stat corrections, with original observations
  python3 scripts/import-catalog-db.py --project-formations # historical loadouts and rule-derived totals
  python3 scripts/import-catalog-db.py --project-formations --check # read-only arrival reference parity

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

from formation_reference import import_arrivals, formation_references

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


def project_unit_types(database, destination, *, check=False):
    """Bounded compatibility projection; other JSON fields are not DB exports."""
    document = read_json(destination)
    require(document["schema_version"] == 1, "Unsupported catalog JSON version")
    allowed_types = {t["id"] for t in document["unit_types"]}
    changed = False
    with closing(connect(database, readonly=True)) as db:
        validate(db)
        for collection in ("entries", "profiles"):
            rows = db.execute("SELECT id,unit_type_id FROM catalog_record WHERE collection=? ORDER BY position", (collection,)).fetchall()
            records = document[collection]
            require([r["id"] for r in records] == [r[0] for r in rows],
                    f"Catalog {collection} record order/IDs differ from DB; reconcile before projection")
            for record, (_, unit_type) in zip(records, rows):
                require(unit_type in allowed_types, f"Type missing from compatibility vocabulary: {unit_type}")
                changed |= record["unit_type"] != unit_type
                record["unit_type"] = unit_type
    if check:
        require(not changed, "Classification projection drift: run python3 scripts/import-catalog-db.py --project-types")
    elif changed:
        # Complete validation precedes replacement. Keep the existing file on failure.
        with tempfile.TemporaryDirectory(prefix=".catalog-types-", dir=destination.parent) as directory:
            candidate = Path(directory) / destination.name
            candidate.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n")
            os.replace(candidate, destination)
    return changed


def project_profile_links(database, destination, *, check=False):
    """Publish only evidence-backed profile links and their curated citation notes.

    Original observations remain immutable in SQLite. The public unknown-CPA
    note is updated, not its value/status, because the old no-profile explanation
    is no longer true. No source_document text or arbitrary details are exported.
    """
    document = read_json(destination)
    require(document["schema_version"] == 1, "Unsupported catalog JSON version")
    before = json_text(document)
    with closing(connect(database, readonly=True)) as db:
        validate(db)
        for collection in ("entries", "profiles"):
            ids = [r[0] for r in db.execute("SELECT id FROM catalog_record WHERE collection=? ORDER BY position", (collection,))]
            require([r["id"] for r in document[collection]] == ids,
                    f"Catalog {collection} record order/IDs differ from DB; reconcile before projection")
        links = {r[0]: r[1:] for r in db.execute("""
            SELECT r.id,r.profile_id,r.note,e.note FROM catalog_record r
            JOIN correction_evidence e ON e.id='profile-link:' || r.id
            WHERE r.collection='entries'
        """)}
        profiles = {r["id"] for r in document["profiles"]}
        sources = {s["id"] for s in document["sources"]}
        for record in document["entries"]:
            if record["id"] not in links:
                continue
            profile, note, observation_note = links[record["id"]]
            require(profile in profiles, f"Reviewed profile missing from compatibility profiles: {profile}")
            stat = record["stats"]["capability_points"]
            require(stat["value"] is None and stat["status"] == "unresolved",
                    f"Reviewed link must not overwrite a resolved CPA: {record['id']}")
            record.update(profile_id=profile, note=note)
            stat["note"] = observation_note
            record["source_refs"] = [r[0] for r in db.execute(
                "SELECT source_id FROM record_source WHERE record_id=? ORDER BY position", (record["id"],))]
            for source_id in record["source_refs"]:
                if source_id in sources:
                    continue
                title, path, details_json = db.execute(
                    "SELECT title,path,details_json FROM source_citation WHERE id=?", (source_id,)).fetchone()
                details = json.loads(details_json)
                citation = {"id": source_id, "title": title, "path": path}
                citation.update({key: details[key] for key in ("section", "pdf_page", "lines", "note") if key in details})
                document["sources"].append(citation)
                sources.add(source_id)
        if links:
            old_gap = "OA pages omit the three Italian divisions Sirte, Marmarica and Cirene that appear on counter p.182. Their 41 counter identities have no inferred characteristic profile."
            document["coverage"]["gaps"] = [
                "Counter-origin identities have base-characteristic links where individually reviewed against VASSAL organization charts; current scenario values remain untranscribed."
                if gap == old_gap else gap for gap in document["coverage"]["gaps"]]
    changed = json_text(document) != before
    if check:
        require(not changed, "Profile projection drift: run python3 scripts/import-catalog-db.py --project-profiles")
    elif changed:
        with tempfile.TemporaryDirectory(prefix=".catalog-profiles-", dir=destination.parent) as directory:
            candidate = Path(directory) / destination.name
            candidate.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n")
            os.replace(candidate, destination)
    return changed


def project_stats(database, destination, *, check=False):
    """Project explicit stat overlays, never a formation's inferred equipment/state.

    Only the public stat fields and citation locators leave the DB. Original
    observations are retained once, from immutable rows, not from prior exports.
    Private source text, evidence quotations and arbitrary details are excluded.
    """
    document = read_json(destination)
    require(document["schema_version"] == 1, "Unsupported catalog JSON version")
    before = json_text(document)
    sources = {s["id"]: s for s in document["sources"]}

    def add_citation(evidence_id, path, locator):
        citation = {"id": evidence_id, "title": "Reviewed stat correction", "path": path,
                    "section": locator, "pdf_page": None, "lines": None,
                    "note": "Original observation retained alongside the correction."}
        if evidence_id in sources:
            require(sources[evidence_id] == citation, f"Correction citation collision: {evidence_id}")
        else:
            document["sources"].append(citation)
            sources[evidence_id] = citation

    with closing(connect(database, readonly=True)) as db:
        validate(db)
        records = {}
        for collection in ("entries", "profiles"):
            ids = [r[0] for r in db.execute("SELECT id FROM catalog_record WHERE collection=? ORDER BY position", (collection,))]
            require([r["id"] for r in document[collection]] == ids,
                    f"Catalog {collection} record order/IDs differ from DB; reconcile before projection")
            records.update({r["id"]: r for r in document[collection]})
        for row in db.execute("""
            SELECT s.record_id,s.stat_id,s.value,s.status,s.raw,s.conditional,s.note,
                   s.original_source_refs_json,s.evidence_id,
                   o.value,o.status,o.raw,o.conditional,o.note,e.source_path,e.locator
            FROM effective_stat s JOIN stat_observation o USING(record_id,stat_id)
            JOIN correction_evidence e ON e.id=s.evidence_id ORDER BY s.record_id,s.stat_id
        """):
            (record_id, stat_id, value, status, raw, conditional, note, refs, evidence_id,
             original_value, original_status, original_raw, original_conditional, original_note, path, locator) = row
            record = records[record_id]
            require(stat_id in record["stats"], f"Corrected stat missing from compatibility record: {record_id}/{stat_id}")
            require(status in ("sourced", "not_applicable", "unresolved"), f"Unsupported corrected status: {status}")
            original = {"value": original_value, "status": original_status, "raw": original_raw,
                        "conditional": bool(original_conditional), "note": original_note,
                        "source_refs": json.loads(refs)}
            record["stats"][stat_id] = {
                "value": value, "status": status, "raw": raw, "conditional": bool(conditional),
                "note": note, "source_refs": [*original["source_refs"], evidence_id],
                "correction_id": evidence_id, "original_observation": original,
            }
            add_citation(evidence_id, path, locator)
        for record_id, missions, note, evidence_id, path, locator, explanation in db.execute("""
            SELECT r.id,json_extract(r.details_json,'$.missions'),r.note,e.id,e.source_path,e.locator,e.note
            FROM catalog_record r JOIN correction_evidence e ON e.id='mission:' || r.id ORDER BY r.id
        """):
            record = records[record_id]
            require(isinstance(missions, str) and 'missions' in record,
                    f"Reviewed mission correction lacks compatibility field: {record_id}")
            record['missions'] = missions
            record['note'] = note + ' ' + explanation
            record['source_refs'] = [r[0] for r in db.execute(
                'SELECT source_id FROM record_source WHERE record_id=? ORDER BY position', (record_id,))] + [evidence_id]
            add_citation(evidence_id, path, locator)
        for record_id, note, evidence_id, path, locator in db.execute("""
            SELECT r.id,r.note,e.id,e.source_path,e.locator FROM catalog_record r
            JOIN correction_evidence e ON e.id='record-note:' || r.id ORDER BY r.id
        """):
            records[record_id]['note'] = note
            if evidence_id not in records[record_id]['source_refs']:
                records[record_id]['source_refs'].append(evidence_id)
            add_citation(evidence_id, path, locator)
        coverage = db.execute("SELECT note FROM correction_evidence WHERE id='catalog-stat-coverage'").fetchone()
        if coverage:
            document['coverage']['gaps'] = [coverage[0] if gap.startswith(('Pz IIIE shifted chart cells,', 'Reviewed errata and ')) else gap
                                            for gap in document['coverage']['gaps']]
        schema_note = db.execute("SELECT note FROM correction_evidence WHERE id='catalog-stat-schema-note'").fetchone()
        if schema_note:
            document['schema_notes'] = [note.replace('Unresolved values are null, never zero.', schema_note[0])
                                        for note in document['schema_notes']]
    changed = json_text(document) != before
    if check:
        require(not changed, "Stat projection drift: run python3 scripts/import-catalog-db.py --project-stats")
    elif changed:
        with tempfile.TemporaryDirectory(prefix=".catalog-stats-", dir=destination.parent) as directory:
            candidate = Path(directory) / destination.name
            candidate.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n")
            os.replace(candidate, destination)
    return changed


def project_formations(database, destination, *, check=False):
    """Add only the reviewed arrival reference and allowlisted citations."""
    document = read_json(destination)
    require(document["schema_version"] == 1, "Unsupported catalog JSON version")
    before = json_text(document)
    with closing(connect(database, readonly=True)) as db:
        validate(db)
        for collection in ("entries", "profiles"):
            ids = [r[0] for r in db.execute("SELECT id FROM catalog_record WHERE collection=? ORDER BY position", (collection,))]
            require([r["id"] for r in document[collection]] == ids,
                    f"Catalog {collection} record order/IDs differ from DB; reconcile before projection")
        references = formation_references(db)
        for record in document["entries"]:
            if record["id"] in references:
                reference = references[record["id"]]
                record["formation_reference"] = reference
                if reference['base_profile_id'] is None:
                    record['note'] = reference['base_profile_note']
                else:
                    record['note'] = record['note'].replace(
                        'Numeric initial TOE, morale, arrival/withdrawal, location, weapon mix and supply state are not transcribed; use the cited page.',
                        'Historical TOE, morale, arrival and weapon assignments are in the cited arrival reference where printed. Current strength, withdrawal, location and supply state are not supplied by that reference.')
        coverage_note = (f'Historical arrival references cover {len(references)} existing formation rows, including printed TOE, morale, arrival and weapon assignments. '
                         'Source blanks, attachments and optional organizations may lack independent numeric quantities. Current strength, withdrawal, placement, readiness and supply inventories require scenario data; these rows are not playable scenario setups.')
        document['coverage']['gaps'] = [coverage_note if gap.startswith(('Initial formation TOE, morale,', 'Historical arrival references cover ')) else gap
                                        for gap in document['coverage']['gaps']]
        sources = {s["id"]: s for s in document["sources"]}
        for source_id, title, path, details in db.execute(
                "SELECT id,title,path,details_json FROM source_citation WHERE id LIKE 'oa-arrival:%' OR id IN ('rules-oa-reference','rules-arrival-raw-strength','rules-arrival-aa-strength','rules-arrival-parenthesized','rules-arrival-naval') ORDER BY id"):
            details = json.loads(details)
            citation = {"id": source_id, "title": title, "path": path,
                        **{k: details[k] for k in ("section", "note", "sha256") if k in details}}
            if source_id in sources:
                require(sources[source_id] == citation, f"Arrival citation collision: {source_id}")
            else:
                document["sources"].append(citation)
                sources[source_id] = citation
        # All profile/equipment evidence must already have its public locator.
        def verify_refs(value):
            if isinstance(value, dict):
                require(set(value.get("source_refs", [])) <= sources.keys(), "Missing public arrival evidence; project reviewed stats first")
                for item in value.values():
                    verify_refs(item)
            elif isinstance(value, list):
                for item in value:
                    verify_refs(item)
        verify_refs(references)
    changed = json_text(document) != before
    if check:
        require(not changed, "Formation projection drift: run python3 scripts/import-catalog-db.py --project-formations")
    elif changed:
        with tempfile.TemporaryDirectory(prefix=".catalog-formations-", dir=destination.parent) as directory:
            candidate = Path(directory) / destination.name
            candidate.write_text(json.dumps(document, ensure_ascii=False, indent=2) + "\n")
            os.replace(candidate, destination)
    return changed


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--database", type=Path, default=DATABASE, help="Private database path (default data/catalog.sqlite)")
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--bootstrap", action="store_true", help="One-time transfer of reviewed JSON into a NEW database")
    modes.add_argument("--patch-only", action="store_true", help="Apply new SQL patches without source/Cargo prerequisites")
    modes.add_argument("--project-types", action="store_true", help="Copy only DB unit_type values into existing public/data/unit-catalog.json; no sources needed")
    modes.add_argument("--project-profiles", action="store_true", help="Copy reviewed DB profile links and citation notes into existing catalog; no numeric values or private texts")
    modes.add_argument("--project-stats", action="store_true", help="Copy reviewed stat overlays and citation locators; preserve original observations and exclude private text")
    modes.add_argument("--project-formations", action="store_true", help="Add reviewed historical arrival references and raw-strength calculations; never current scenario values")
    parser.add_argument("--check", action="store_true", help="Verify the selected import/projection causes no change; never write")
    args = parser.parse_args()
    if args.bootstrap and args.check:
        parser.error("--bootstrap cannot be combined with --check")
    try:
        require(sqlite3.sqlite_version_info >= (3, 37), "SQLite 3.37+ required")
        if args.project_types:
            changed = project_unit_types(args.database, ROOT / "public/data/unit-catalog.json", check=args.check)
            print(json.dumps({"projection": "unit_type_only", "changed": changed}))
            return
        if args.project_profiles:
            changed = project_profile_links(args.database, ROOT / "public/data/unit-catalog.json", check=args.check)
            print(json.dumps({"projection": "reviewed_profile_links", "changed": changed}))
            return
        if args.project_stats:
            changed = project_stats(args.database, ROOT / "public/data/unit-catalog.json", check=args.check)
            print(json.dumps({"projection": "reviewed_stats", "changed": changed}))
            return
        if args.project_formations:
            changed = project_formations(args.database, ROOT / "public/data/unit-catalog.json", check=args.check)
            print(json.dumps({"projection": "formation_arrival_reference", "changed": changed}))
            return
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
            import_arrivals(db, ROOT)

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
