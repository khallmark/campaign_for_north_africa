#!/usr/bin/env python3
"""Build reference data from the existing VASSAL census, not another census.

python scripts/import-unit-data.py --ocr   # optional, slow source-text extraction
python scripts/import-unit-data.py         # deterministic structured import
python scripts/import-unit-data.py --check # read-only output/source drift check

Requires Python 3. OCR additionally requires tesseract and pdftotext. Neither
mode reads saved games, publishes source images, or changes simulation stats.
"""

import argparse
import copy
import hashlib
import json
import re
import subprocess
import sys
import xml.etree.ElementTree as ET
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.parse import quote


ROOT = Path(__file__).resolve().parents[1]
MODULE = Path("docs/CNAv2.1.0")
CENSUS = Path("data/sources/vassal-2.1-pieces.json")
EVIDENCE = Path("data/sources/unit-reference-evidence.json")
TEXT = Path("data/sources/vassal-2.1-document-text.json")
REFERENCE = Path("public/data/unit-reference.json")
MATRIX = Path("data/sources/identity-matrix.json")


def read_json(path):
    return json.loads(path.read_text())


def serialized(value):
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def source_info(root, path, locks):
    path = Path(path)
    full = root / path
    require(full.resolve().is_relative_to(root.resolve()), f"Source escapes repository: {path}")
    data = full.read_bytes()
    require(not data.startswith(b"version https://git-lfs.github.com/spec/v1"),
            f"Hydrate Git LFS source: {path}")
    digest = hashlib.sha256(data).hexdigest()
    if str(path) in locks:
        require(locks[str(path)] == {"size": len(data), "sha256": digest},
                f"Locked source changed: {path}; review rather than updating the lock")
    return {"source_path": str(path), "sha256": digest, "size": len(data)}


def link_identities(inventory, census, evidence):
    """Only reviewed multi-source anchors and census-evidenced palette copies link."""
    identities = {i["id"]: i for i in inventory["entries"]}
    slots = {s["gpid"]: s for s in census["slots"]}
    links = {}
    for group in evidence["link_groups"]:
        for suffix, (gpid, label) in group["units"].items():
            unit_id = f"unit-{group['nationality']}-{suffix}"
            identity, slot = identities[unit_id], slots[gpid]
            require(slot["palette"] == "organisation_charts", f"Not an OC anchor: {gpid}")
            require(slot["entry_name"] == label, f"Counter label drift: {gpid}")
            require(f"VASSAL.build.widget.WidgetMap:{group['vassal_board']}" in slot["palette_path"],
                    f"Counter board mismatch: {gpid}")
            require(identity.get("formation_context") == group["formation_context"],
                    f"Canonical formation mismatch: {unit_id}")
            require(group["manual_source"] in identity["source_refs"], f"Manual citation mismatch: {unit_id}")
            require(slot["side"] == identity["side"], f"Side mismatch: {gpid}")
            require(gpid not in links, f"Competing counter assignment: {gpid}")
            links[gpid] = {"unit_id": unit_id, "status": "reviewed_source_link",
                           "anchor_gpid": gpid, "manual_source": group["manual_source"],
                           "board": group["vassal_board"]}
    for slot in census["slots"]:
        if slot["gpid"] in links:
            continue
        anchors = [g for g in slot["duplicate_palette_candidates"] if g in links]
        if not anchors:
            continue
        assignments = {links[g]["unit_id"] for g in anchors}
        require(len(assignments) == 1, f"Ambiguous palette copy: {slot['gpid']}")
        for gpid in anchors:
            anchor = slots[gpid]
            require(slot["entry_name"] == anchor["entry_name"]
                    and Path(slot["basic_image"]).stem == Path(anchor["basic_image"]).stem
                    and slot["side"] == anchor["side"], f"Invalid census palette link: {slot['gpid']}")
        links[slot["gpid"]] = {**links[anchors[0]], "status": "evidenced_palette_copy"}
    return links


def documents(root, locks):
    """Index every chart and OC board, excluding the unrelated full-map raster."""
    xml = ET.parse(root / MODULE / "buildFile.xml")
    docs = {}
    for element in xml.iter():
        tag = element.tag.rsplit(".", 1)[-1]
        if tag == "Chart":
            name, title = element.get("fileName"), element.get("chartName")
        elif tag == "Board" and element.get("image", "").startswith("OC "):
            name, title = element.get("image"), element.get("name")
        else:
            continue
        if not name:
            continue
        path = MODULE / "images" / name
        require(Path(name).name == name, f"Unsafe document name: {name}")
        if str(path) not in docs:
            docs[str(path)] = {**source_info(root, path, locks), "kind": "chart_image", "titles": []}
        if title not in docs[str(path)]["titles"]:
            docs[str(path)]["titles"].append(title)
    for path in sorted((root / MODULE).iterdir()):
        if path.suffix.lower() not in {".txt", ".pdf"}:
            continue  # Never decode .vsav files or credentials.
        rel = path.relative_to(root)
        docs[str(rel)] = {**source_info(root, rel, locks), "kind": "pdf" if path.suffix == ".pdf" else "text",
                          "titles": [path.name]}
    return list(docs.values())


def extract_documents(root, docs, workers):
    cache = read_json(root / TEXT) if (root / TEXT).exists() else {"documents": []}
    old = {d["source_path"]: d for d in cache["documents"]}
    tesseract = subprocess.run(["tesseract", "--version"], capture_output=True, text=True, check=True).stdout.splitlines()[0]
    pdf_version = subprocess.run(["pdftotext", "-v"], capture_output=True, text=True, check=True).stderr.splitlines()[0]

    def extract(doc):
        tool = tesseract if doc["kind"] == "chart_image" else pdf_version if doc["kind"] == "pdf" else "python-text-latin1"
        previous = old.get(doc["source_path"], {})
        if previous.get("sha256") == doc["sha256"] and previous.get("tool") == tool:
            return previous
        path = root / doc["source_path"]
        if doc["kind"] == "chart_image":
            proc = subprocess.run(["tesseract", str(path), "stdout", "--psm", "3"],
                                  capture_output=True, text=True, check=True)
            text, status = proc.stdout, "ocr_unverified"
        elif doc["kind"] == "pdf":
            proc = subprocess.run(["pdftotext", "-layout", str(path), "-"],
                                  capture_output=True, text=True, check=True)
            text, status = proc.stdout, "text_layer_unverified"
        else:
            text, status = path.read_text(encoding="latin1"), "source_text"
        print(f"Extracted {doc['source_path']}", file=sys.stderr, flush=True)
        return {**doc, "tool": tool, "status": status, "text": text,
                "note": "Search/transcription aid only. Never automatically promoted to numeric stats. PDF image-only pages require chart OCR or visual review."}

    with ThreadPoolExecutor(max_workers=workers) as pool:
        results = list(pool.map(extract, docs))
    return {"schema_version": 1, "documents": results}


def apply_evidence(root, catalog, evidence, locks):
    records = copy.deepcopy(catalog["entries"])
    profiles = copy.deepcopy(catalog["profiles"])
    by_id = {r["id"]: r for r in records + profiles}
    checks = []
    for group in evidence["profile_cpa_checks"]:
        image = source_info(root, MODULE / "images" / group["vassal_image"], locks)
        for pair in group["values"].split(";"):
            code, raw = pair.split("=")
            record = by_id[f"{group['prefix']}-{code}"]
            stat = record["stats"]["capability_points"]
            value = int(re.match(r"\d+", raw)[0])
            check = {"record_id": record["id"], "stat": "capability_points",
                     "vassal_raw": raw, "vassal_value": value, "manual_catalog_value": stat["value"],
                     "manual_catalog_raw": stat["raw"], "manual_pages": group["manual_pages"],
                     "vassal_source": image, "scope": group["scope"],
                     "result": "numeric_agreement" if stat["value"] == value else "conflict"}
            stat["cross_check"] = check
            checks.append(check)
    require(len(checks) == len({c["record_id"] for c in checks}) == len(profiles),
            "CPA checks must cover every profile once")
    applied = set()
    for erratum in evidence["errata"]:
        require(erratum["id"] not in applied, f"Repeated erratum: {erratum['id']}")
        applied.add(erratum["id"])
        source = source_info(root, erratum["source_path"], locks)
        text = (root / erratum["source_path"]).read_text()
        require(erratum["quote"] in text, f"Erratum quote missing: {erratum['id']}")
        for key, value in erratum["values"].items():
            record = by_id[erratum["record_id"]]
            original = copy.deepcopy(record["stats"][key])
            require("erratum_id" not in original, f"Two errata target one cell: {record['id']}/{key}")
            record["stats"][key] = {
                "value": value, "status": "not_applicable" if value is None else "sourced",
                "raw": "-" if value is None else str(value),
                "conditional": original["conditional"] or key in erratum.get("conditional_fields", []),
                "source_refs": [erratum["id"]], "note": erratum["note"], "erratum_id": erratum["id"],
                "original_observation": original, "verification": "explicit_vassal_erratum",
                "errata_source": {**source, "locator": erratum["locator"], "quote": erratum["quote"]}}
    return records, profiles, checks


def build(root):
    locks = read_json(root / "docs/sources.lock.json")["files"]
    source_info(root, MODULE / "buildFile.xml", locks)
    # Existing source catalog and census are inputs, never rewritten by this importer.
    catalog = read_json(root / "public/data/unit-catalog.json")
    inventory = read_json(root / "public/data/unit-identities.json")
    census = read_json(root / CENSUS)
    evidence = read_json(root / EVIDENCE)
    links = link_identities(inventory, census, evidence)
    records, profiles, checks = apply_evidence(root, catalog, evidence, locks)
    record_map = {r["id"]: r for r in records}
    assets = {}
    counters = []
    matrix_slots = []
    for slot in census["slots"]:
        image_ids = []
        for image in slot["images"]:
            name = image["name"]
            require(Path(name).name == name and "\\" not in name, f"Unsafe image name: {name}")
            if name not in assets:
                path = MODULE / "images" / name
                exists = (root / path).is_file()
                info = source_info(root, path, locks) if exists else {"source_path": str(path), "sha256": None, "size": None}
                assets[name] = {**info, "id": name, "exists": exists, "kind": "vassal_source_image",
                                "public_path": None, "private_review_path": "/__source-tokens/" + quote(name, safe="") if exists else None,
                                "distribution": "private_review_only_public_release_pending"}
            require(assets[name]["exists"] == image["exists"], f"Census asset drift: {name}")
            image_ids.append(name)
        association = links.get(slot["gpid"])
        counter = {"gpid": slot["gpid"], "canonical_unit_id": association["unit_id"] if association else None,
                   "label": slot["entry_name"], "kind": slot["slot_kind"], "side": slot["side"],
                   "nationality_prefix": slot["nationality_prefix"], "palette": slot["palette"],
                   "palette_path": slot["palette_path"], "source_line": slot["source_line"],
                   "basic_image_id": slot["basic_image"] or None, "image_ids": list(dict.fromkeys(image_ids)),
                   "layers": slot["layers"], "duplicate_palette_candidates": slot["duplicate_palette_candidates"],
                   "link_status": association["status"] if association else "unresolved"}
        require(counter["basic_image_id"] is None or counter["basic_image_id"] in assets,
                f"Base image absent from census images: {slot['gpid']}")
        counters.append(counter)
        matrix_slots.append({"gpid": slot["gpid"], "unit_id": counter["canonical_unit_id"],
                             "status": counter["link_status"], "evidence": association,
                             "source_line": slot["source_line"], "palette": slot["palette"],
                             "label": slot["entry_name"], "basic_image": slot["basic_image"],
                             "reason": None if association else "No reviewed multi-source canonical link; equal labels alone are insufficient."})
    organization = evidence["organization_checks"]
    org_image = source_info(root, MODULE / "images" / organization["vassal_image"], locks)
    units = []
    for identity in inventory["entries"]:
        source_records = [record_map[r] for r in identity["source_entry_ids"]]
        counter_ids = [s["gpid"] for s in counters if s["canonical_unit_id"] == identity["id"]]
        org_row = organization["rows"].get(identity["id"].removeprefix("unit-commonwealth-"))
        org = None
        if org_row and identity.get("formation_context") == organization["formation_context"]:
            org = {"context": "organization_at_arrival", "equipment_toe_raw": org_row[0], "arrival_raw": org_row[1],
                   "basic_morale_raw": organization["basic_morale_raw"], "status": "visually_cross_checked",
                   "source": org_image, "manual_source": organization["manual_source"],
                   "scope": organization["scope"], "disagreements": organization["disagreements"]}
        units.append({"id": identity["id"], "name": identity["name"], "side": identity["side"],
                      "nationality": identity["nationality"], "entity_kind": identity["entity_kind"],
                      "identity_status": identity["identity_status"], "formation_context": identity.get("formation_context"),
                      "unit_type_ids": sorted({r["unit_type"] for r in source_records}),
                      "source_record_ids": identity["source_entry_ids"],
                      "characteristic_profile_ids": sorted({r["profile_id"] for r in source_records if r.get("profile_id")}),
                      "source_refs": identity["source_refs"], "counter_gpids": counter_ids,
                      "token_link_status": "linked" if counter_ids else "unresolved",
                      "generated_art": copy.deepcopy(identity["sprite"]), "organization_at_arrival": org,
                      "description": {"text": organization["description"], "status": organization["description_status"],
                                      "scope": "formation_history_not_individual_unit", "source": org_image} if org else None})
    source_docs = documents(root, locks)
    extracted = read_json(root / TEXT) if (root / TEXT).exists() else {"documents": []}
    text_by_path = {d["source_path"]: d for d in extracted["documents"]}
    for doc in source_docs:
        extracted_doc = text_by_path.get(doc["source_path"])
        if extracted_doc:
            require(extracted_doc["sha256"] == doc["sha256"], f"Stale extracted document: {doc['source_path']}")
        doc["text_status"] = extracted_doc["status"] if extracted_doc else "not_extracted"
        doc["text_index_path"] = str(TEXT) if extracted_doc else None
    coverage = {"complete": False, "canonical_candidates": len(units), "vassal_slots": len(counters),
                "linked_canonical_candidates": sum(bool(u["counter_gpids"]) for u in units),
                "unresolved_canonical_candidates": sum(not u["counter_gpids"] for u in units),
                "linked_vassal_slots": len(links), "unresolved_vassal_slots": len(counters) - len(links),
                "source_images": len(assets), "missing_source_images": sum(not a["exists"] for a in assets.values()),
                "generated_art_ready": sum(u["generated_art"]["status"] == "ready" for u in units),
                "source_records": len(records), "characteristic_profiles": len(profiles),
                "profile_cpa_numeric_agreements": sum(c["result"] == "numeric_agreement" for c in checks),
                "organization_rows_cross_checked": sum(u["organization_at_arrival"] is not None for u in units),
                "errata_applied_once": len(evidence["errata"]), "indexed_documents": len(source_docs),
                "extracted_documents": sum(d["text_status"] != "not_extracted" for d in source_docs)}
    inputs = [CENSUS, EVIDENCE, Path("public/data/unit-catalog.json"), Path("public/data/unit-identities.json")]
    if (root / TEXT).exists():
        inputs.append(TEXT)
    for source in catalog["source_files"]:
        info = source_info(root, source["path"], locks)
        require(info["sha256"] == source["sha256"], f"Catalog source changed: {source['path']}")
        inputs.append(Path(source["path"]))
    sources = copy.deepcopy(catalog["sources"])
    sources.extend({"id": e["id"], "title": "VASSAL incorporated erratum", "path": e["source_path"],
                    "section": e["locator"], "pdf_page": None, "lines": None, "note": e["note"]}
                   for e in evidence["errata"])
    reference = {"schema_version": 1, "purpose": "Reference data only; no scenario or sandbox state.",
                 "coverage": coverage, "input_sources": [source_info(root, p, locks) for p in inputs],
                 "identity_redirects": inventory["identity_redirects"], "units": units, "counters": counters,
                 "token_assets": list(assets.values()), "unit_types": catalog["unit_types"],
                 "stat_definitions": catalog["stat_definitions"], "sources": sources,
                 "records": records, "profiles": profiles, "cpa_checks": checks, "errata": evidence["errata"],
                 "documents": source_docs}
    matrix = {"schema_version": 1, "coverage": coverage,
              "canonical_units": [{"id": u["id"], "source_record_ids": u["source_record_ids"],
                                   "source_refs": u["source_refs"], "counter_gpids": u["counter_gpids"],
                                   "status": u["token_link_status"]} for u in units],
              "vassal_slots": matrix_slots, "identity_redirects": inventory["identity_redirects"]}
    return reference, matrix


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="Validate source hashes and deterministic output without writing")
    mode.add_argument("--ocr", action="store_true", help="Extract every VASSAL chart/OC board and document before import (slow)")
    parser.add_argument("--workers", type=int, default=4, help="OCR concurrency,1-8 (default4)")
    args = parser.parse_args()
    if not 1 <= args.workers <= 8:
        parser.error("--workers must be between1 and8")
    try:
        if args.ocr:
            locks = read_json(ROOT / "docs/sources.lock.json")["files"]
            (ROOT / TEXT).write_text(serialized(extract_documents(ROOT, documents(ROOT, locks), args.workers)))
        reference, matrix = build(ROOT)
        for path, value in [(REFERENCE, reference), (MATRIX, matrix)]:
            text = serialized(value)
            if args.check:
                require((ROOT / path).is_file() and (ROOT / path).read_text() == text,
                        f"Generated data drift: {path}; run python scripts/import-unit-data.py")
            else:
                (ROOT / path).write_text(text)
        print(serialized(reference["coverage"]), end="")
    except (OSError, ValueError, KeyError, ET.ParseError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Unit import failed: {error}\n")


if __name__ == "__main__":
    main()
