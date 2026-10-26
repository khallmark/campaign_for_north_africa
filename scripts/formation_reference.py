"""Reviewed OA observations and bounded historical-reference projection.

Never updates identities, profile assignments or current scenario observations.
Only exact chart vocabulary aliases are accepted; no fuzzy name matching.
"""

import copy
import hashlib
import json
import re


ALIASES = {
    "cw": {
        "a9-cruiser": ["A9", "A9 Cruiser", "A9 Cruisers"],
        "a10-cruiser": ["A10", "A10 Cruiser", "A10 Cruisers"],
        "a13-cruiser": ["A13", "A13 Cruiser", "A13 Cruisers"],
        "crusader-mk-i": ["Crusader Mk I", "Crusader I"],
        "crusader-mk-ii": ["Crusader Mk II"],
        "grant-m3": ["Grant"], "stuart-m3": ["Stuart", "Stuarts"],
        "mark-vi-light": ["Mk VI Lights", "Mark VI Light", "Mark VI Lights"],
        "matilda-mk-ii": ["Matilda"], "valentine-mk-ii": ["Valentine"],
        "18-pounder-gun": ["18-pounder"],
        "18-25-pounder-gun": ["18/25-pounder", "18/25-pounders"],
        "25-pounder-gun": ["25-pounder", "25-pounders"],
        "2-pounder": ["2-pounder", "2-pounders"],
        "6-pounder": ["6-pounder", "6-pounders"],
        "4-5-inch-gun": ["4.5-inch Gun"],
        "5-5-inch-gun-howitzer": ["5.5-inch Gun-Howitzer"],
        "60-pounder-gun": ["60-pounder"],
        "sp-25-pounder-gun-bishop": ["SP 25-pounder"],
        "3-7-inch-howitzer": ["3.7-inch Howitzer"],
        "4-5-inch-howitzer": ["4.5-inch Howitzer"],
        "6-inch-howitzer": ["6-inch Howitzer"],
        "155mm-howitzer": ["155mm Howitzer"],
        "105mm-sp-howitzer-priest": ["105mm Howitzer (SPA)"],
        "light-aa-bofors-40mm": ["Light AA"], "heavy-aa-3-7-inch": ["Heavy AA"],
    },
    "ge": {
        "pz-i": ["PzI"], "pz-ii": ["PzII"], "pz-iii-e": ["PzIIIE"],
        "pz-iii-h": ["PzIIIH"], "pz-iv-d": ["PzIVD"], "pz-iv-e": ["PzIVE"],
        "7-5cm-ig18-light-infantry-gun": ["7.5cm Lt Inf Gun", "7.5cm Lt Gun"],
        "10-5cm-k18-medium-gun": ["10.5cm Med Gun"],
        "10-5cm-lefh18-light-field-howitzer": ["10.5cm Lt Field Howitzer"],
        "15cm-sig33-medium-infantry-gun": ["15cm Med Inf Gun"],
        "15cm-sfh18-medium-field-howitzer": ["15cm Med Field Howitzer"],
        "15cm-k18-gun": ["15cm Gun", "15cm Med Gun"],
        "15cm-sp-gun": ["15cm SP Gun"],
        "155mm-french-1915-short": ["155mm (Fr. 1915 short)"],
        "17cm-k18-gun": ["17cm Gun"], "21cm-mrs18-howitzer": ["21cm How", "21cm Howitzer"],
        "3-7cm-pak-35-36": ["3.7cm"], "5cm-pak-38": ["5cm"], "pzjg-i-sp": ["PzjgI"],
        "light-aa-20mm-and-37mm-flak": ["Light AA"], "heavy-aa-88mm-flak": ["Heavy AA"],
    },
    "it": {
        "cv-33-l3-35": ["CV33"], "l6-40": ["L6/40"], "m11-39": ["M11/39"],
        "m13-40": ["M13/40"], "m14-41": ["M14/41"],
        "65-17-gun": ["65/17 Gun", "65/17"], "75-18-gun-howitzer": ["75/18 Gun-Howitzer"],
        "75-18-gun-semovente": ["Semovente 75/18 SP Gun", "75/18 SP Gun"],
        "75-27-gun": ["75/27 Gun"], "100-17-howitzer": ["100/17 Howitzer"],
        "105-28-gun": ["105/28 Gun"], "149-13-howitzer": ["149/13 Howitzer"],
        "paraart": ["ParaArt"], "149mm-vichy-french": ["149mm Vichy French Gun"],
        "47-32-mod-37": ["47/32 Gun", "47/32"],
        "light-aa-20mm-m-35-breda": ["Light AA"], "i-light-aa": ["'I' Light AA"],
        "heavy-aa-75-46-mod-34": ["Heavy AA", "75/46 AA Gun"], "i-heavy-aa": ["'I' Heavy AA"],
    },
}
WEAPONS = {(nationality, label): f"weapons-{nationality}-{suffix}"
           for nationality, records in ALIASES.items() for suffix, labels in records.items() for label in labels}
# The Free French sheet expressly directs the reader to Italian artillery.
WEAPONS["cw", "French 149mm (see It. Arty)"] = "weapons-it-149mm-vichy-french"
WEAPONS["it", "vv"] = "it-vv"  # Printed ID code, not a gun-model name.
COMBAT_STATS = ("anti_air", "barrage", "anti_armor", "close_assault_offensive", "close_assault_defensive")
RULES = {
    "rules-oa-reference": "3.5, 4.42, 4.45–4.46: OA quantities, characteristics and assigned weapon systems",
    "rules-arrival-raw-strength": "11.31–11.38: rating × TOE gives Raw Points; combine before conversion/rounding",
    "rules-arrival-aa-strength": "11.37, 46.11: AA rating × TOE gives Actual AA Points, without dividing by ten",
    "rules-arrival-parenthesized": "3.3–3.4, 4.46 and national chart legends: parenthesized combat ratings; ammunition and other friendly units restrict use",
    "rules-arrival-naval": "30.12–30.17, 30.39; Tobruk OA note c: naval points, damage and immobile San Giorgio; not land TOE",
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def source_key(record_id):
    return "-".join(record_id.split("-")[:3]) if record_id.startswith("oa-") else record_id


def import_arrivals(db, root):
    """Import all reviewed rows into private DB details, with locked image evidence."""
    document = json.loads((root / "data/sources/formation-arrivals.json").read_text())
    require(document["schema_version"] == 1, "Unsupported arrival transcription version")
    records = {r[0]: r for r in db.execute("SELECT id,kind,nationality,profile_id,details_json FROM catalog_record")}
    formations = {k: v for k, v in records.items() if v[1] in ("formation", "counter_formation")}
    keys = {source_key(k): k for k in formations}
    require(len(keys) == len(formations), "Ambiguous formation source-row key")
    require(set(document.get("row_options", {})) <= keys.keys(), "Unknown arrival row-options key")
    seen = set()
    for sheet in document["sheets"]:
        image = sheet["image"]
        require("/" not in image and "\\" not in image, f"Unsafe arrival image: {image}")
        path = "docs/CNAv2.1.0/images/" + image
        locked = db.execute("SELECT sha256 FROM source_file WHERE path=?", (path,)).fetchone()
        require(locked and hashlib.sha256((root / path).read_bytes()).hexdigest() == locked[0],
                f"Arrival source hash mismatch: {path}")
        row_keys = {r[0] for r in sheet["rows"]}
        for field in ("morale_overrides", "arrival_profile_overrides", "reviewed_profile_disagreements", "row_references"):
            require(set(sheet.get(field, {})) <= row_keys, f"Unknown {field} key in {image}")
        for key, code, toe, arrives, note in sheet["rows"]:
            require(key in keys and key not in seen, f"Unknown/repeated arrival row: {key}")
            seen.add(key)
            record_id = keys[key]
            _, _, nationality, old_profile, details_json = formations[record_id]
            details = json.loads(details_json)
            require(details.get("formation") == sheet["formation"], f"Arrival formation mismatch: {key}")
            prefix = {"commonwealth": "cw", "german": "ge", "italian": "it"}[nationality]
            printed_profile = f"{prefix}-{code}" if code else None
            previous = sheet.get("reviewed_profile_disagreements", {}).get(key, printed_profile)
            require(old_profile == previous, f"Unreviewed arrival profile disagreement: {key}: {old_profile}/{printed_profile}")
            profile = sheet.get("arrival_profile_overrides", {}).get(key, printed_profile)
            require(profile is None or profile in records and records[profile][1] == "profile", f"Unknown arrival profile: {profile}")
            reference = sheet.get("row_references", {}).get(key)
            reference = keys.get(reference, reference)
            require(reference is None or reference in records and records[reference][1] in ("formation", "counter_formation", "naval_unit")
                    and records[reference][2] == nationality and reference != record_id,
                    f"Invalid same-unit reference: {reference}")
            components = []
            for part in (toe or "").replace(" + ", ", ").split(", "):
                if part in ("", "-"):
                    continue
                match = re.fullmatch(r"([UO])@(\d+)(?: TOE)?", part)
                if part == "N" or match:
                    components.append({"record_id": profile, "label": part, "normal": part == "N",
                                       "toe": int(match[2]) if match else None, "land_barrage_toe": None})
                    continue
                match = re.fullmatch(r"(\d+) x (?:\((\d+)\) )?(.+)", part)
                require(match is not None, f"Unparsed arrival loadout: {key}: {part}")
                target = WEAPONS.get((prefix, match[3]))
                require(target in records, f"Unreviewed equipment alias: {prefix}/{match[3]}")
                components.append({"record_id": target, "label": match[3], "normal": False,
                                   "toe": int(match[1]), "land_barrage_toe": int(match[2]) if match[2] else None})
            citation = "oa-arrival:" + key
            citation_details = {"section": f"{sheet['formation']}; {record_id}; ID Code / TOE and Weapons Systems / Arrives",
                                "note": "Visually reviewed arrival reference, not current scenario state.",
                                "sha256": locked[0]}
            db.execute("INSERT INTO source_citation VALUES (?,?,?,?) ON CONFLICT(id) DO UPDATE SET title=excluded.title,path=excluded.path,details_json=excluded.details_json",
                       (citation, "Organization at Arrival: " + sheet["formation"], path, json.dumps(citation_details)))
            details["formation_arrival"] = {
                "profile_id": profile, "printed_profile_code": code, "previous_profile_id": old_profile,
                "toe_raw": toe, "arrives_raw": arrives,
                "morale_raw": sheet.get("morale_overrides", {}).get(key, sheet["morale_raw"]),
                "note": " ".join(filter(None, (sheet.get("note"), note))),
                "source_refs": [citation, "rules-oa-reference"], "components": components,
                "related_record_id": reference,
                "options": document.get("row_options", {}).get(key, {}),
            }
            db.execute("UPDATE catalog_record SET details_json=? WHERE id=?", (json.dumps(details, sort_keys=True), record_id))
    require(seen == set(keys), f"Arrival rows missing: {sorted(set(keys) - seen)}")
    for rule, section in RULES.items():
        db.execute("INSERT INTO source_citation VALUES (?,?,?,?) ON CONFLICT(id) DO UPDATE SET details_json=excluded.details_json",
                   (rule, "CNA reference calculation rules", "docs/rules.txt",
                    json.dumps({"section": section, "note": "Historical reference calculation; not current scenario strength."})))
    db.commit()


def stat(value, raw, refs, note, *, conditional=False, unit=None):
    result = {"value": value, "raw": raw, "status": "sourced" if value is not None else "unresolved",
              "conditional": conditional, "source_refs": list(dict.fromkeys(refs)), "note": note}
    if unit:
        result["unit"] = unit
    return result


def formation_references(db):
    """Return only public typed reference fields, computed from DB observations."""
    units = dict(db.execute("SELECT id,unit FROM stat_definition"))
    records = {r[0]: {"name": r[1], "kind": r[2], "note": r[3], "details": json.loads(r[4]), "unit_type": r[5], "stats": {}}
               for r in db.execute("SELECT id,name,kind,note,details_json,unit_type_id FROM catalog_record")}
    for record_id, stat_id, value, status, raw, conditional, note, refs, evidence in db.execute(
            "SELECT record_id,stat_id,value,status,raw,conditional,note,original_source_refs_json,evidence_id FROM effective_stat"):
        records[record_id]["stats"][stat_id] = {
            "value": value, "status": status, "raw": raw, "conditional": bool(conditional), "note": note,
            "source_refs": json.loads(refs) + ([evidence] if evidence else []), "unit": units[stat_id],
        }
    result = {}
    for record_id, record in records.items():
        if record["kind"] not in ("formation", "counter_formation"):
            continue
        require("formation_arrival" in record["details"], f"No imported arrival reference: {record_id}")
        arrival = record["details"]["formation_arrival"]
        refs = arrival["source_refs"]
        profile_id = arrival["profile_id"]
        related = records.get(arrival["related_record_id"])
        related_arrival = related["details"].get("formation_arrival") if related else None
        profile_note = None
        # A reviewed same-unit reference can supply characteristics, never a
        # second arrival/loadout or the attachment's later current strength.
        if profile_id is None and related_arrival and related_arrival["profile_id"]:
            profile_id = related_arrival["profile_id"]
            profile_note = (f"Base characteristics from the same unit's cited row {arrival['related_record_id']}. "
                            "This attachment row prints no ID code; its blank TOE is not a second loadout. " + arrival["note"])
        profile = records.get(profile_id)
        base = copy.deepcopy(profile["stats"]) if profile else {}
        if profile_note:
            for rating in base.values():
                rating["source_refs"] = list(dict.fromkeys([*rating["source_refs"], *refs, *related_arrival["source_refs"]]))
        options = arrival["options"]
        # These row-specific truck notes change the printed base marker, not CP
        # remaining or the numeric motorized allowance.
        if "base_cpa_raw" in options:
            require(options["base_cpa_raw"] == "10+", "Unsupported reviewed base-CPA override")
            base["capability_points"] = stat(10, options["base_cpa_raw"], refs, arrival["note"], unit="Capability points")
        components = []
        for item in arrival["components"]:
            source = records.get(item["record_id"])
            ratings = copy.deepcopy(source["stats"]) if source else {}
            quantity = item["toe"]
            quantity_refs = list(refs)
            conditional = False
            quantity_note = "Printed TOE strength points; not vehicle count."
            if item["normal"]:
                maximum = ratings.get("maximum_toe", {})
                quantity_note = "Normal arrival uses profile maximum (4.45); not vehicle count."
                if maximum.get("status") == "sourced":
                    quantity = maximum["value"]
                    conditional = maximum["conditional"]
                    quantity_refs += maximum["source_refs"]
                else:
                    quantity_note = f"Arrival prints N, but {item['record_id']} supplies no numeric whole-unit maximum (printed {maximum.get('raw') or 'blank'}). " + maximum.get('note', '')
            if options.get("calculation_note"):
                quantity = None
            components.append({
                "record_id": item["record_id"], "name": source["name"] if source else item["label"],
                "label": item["label"], "kind": "equipment" if source and source["kind"] == "equipment" else "profile",
                "toe": stat(quantity, item["label"] if item["normal"] else str(item["toe"]) if item["toe"] is not None else None,
                            quantity_refs, options.get("calculation_note") or quantity_note,
                            conditional=conditional, unit="TOE strength points"),
                "per_toe_stats": ratings, "land_barrage_toe": item["land_barrage_toe"],
            })
        totals = {}
        for key in COMBAT_STATS:
            rule = "rules-arrival-aa-strength" if key == "anti_air" else "rules-arrival-raw-strength"
            total_refs = [*refs, rule]
            terms, reasons, conditions = [], [], []
            total = 0
            excluded_assault = False
            if options.get("calculation_note"):
                reasons.append(options["calculation_note"])
            if key == "close_assault_offensive" and options.get("no_assault"):
                reasons.append("Emplaced guns may not assault; see the OA note. No offensive total inferred from generic gun ratings.")
            parenthesized_unit = bool(components and re.fullmatch(r"\(\d+\)", base.get("maximum_toe", {}).get("raw") or ""))
            if not components:
                reasons.append("No independent quantified loadout is printed. " + arrival['note'])
            contributions = []
            for item in components:
                rating = item["per_toe_stats"].get(key, {})
                quantity = item["toe"]["value"]
                if key == "barrage" and item["land_barrage_toe"] is not None:
                    quantity = item["land_barrage_toe"]
                    conditions.append("Only the printed land-target subset contributes to this barrage total.")
                total_refs += item["toe"]["source_refs"] + rating.get("source_refs", [])
                if item["record_id"] == "it-g":
                    reasons.append("Profile g: no assigned weapon model is printed (4.46).")
                    continue
                if (options.get("no_assault") and key in ("close_assault_offensive", "close_assault_defensive")
                        and records[item["record_id"]]["unit_type"] in ("artillery", "airborne-artillery")):
                    excluded_assault = True
                    conditions.append("Emplaced artillery TOE cannot participate in offensive or defensive assault under the cited OA note. Exclude those guns before selecting eligible normal or parenthesized fire; non-emplaced AA retains its own restrictions.")
                    continue
                value = rating.get("value") if rating.get("status") == "sourced" else None
                # A known N/A combat cell supplies no intrinsic points. This
                # applies only to a specified component, never an unknown
                # loadout, maximum TOE, vulnerability or armor protection.
                # Preserve the original nullable per-TOE chart cell.
                if rating.get("status") == "not_applicable" and rating.get("raw") == "-":
                    value = 0
                if quantity is None or value is None:
                    if quantity is None:
                        reasons.append(f"{item['label']}: {item['toe']['note']}")
                    if value is None:
                        reasons.append(f"{item['label']}: {key} prints {rating.get('raw') or 'no cell'}, not a single numeric rating. {rating.get('note', '')}")
                    continue
                parenthesized_rating = bool(re.fullmatch(r"\(\d+\)", rating.get("raw") or ""))
                parenthesized = parenthesized_unit or parenthesized_rating
                date_applied = False
                if key == "anti_armor" and rating.get("conditional") and item["record_id"] in (
                        "weapons-it-75-27-gun", "weapons-ge-7-5cm-ig18-light-infantry-gun"):
                    date = re.fullmatch(r"[123]/(\d+)", arrival["arrives_raw"] or "")
                    turn = int(date[1]) if date else 1 if arrival["arrives_raw"] == "D" else None
                    if turn is not None:
                        value = 0 if turn < 63 else value
                        date_applied = True
                        conditions.append(f"{item['label']}: anti-armor {value:g} at printed arrival {arrival['arrives_raw']}; "
                                          "chart footnote ** changes the rating at 1/63. Not a rating for every scenario date.")
                if rating.get("conditional") and not parenthesized_rating and not date_applied:
                    reasons.append(f"{item['label']}: {rating.get('raw')}; {rating.get('note', '')}")
                    conditions.append(reasons[-1])
                if item["toe"]["conditional"] and not re.fullmatch(r"\(\d+\)", item["per_toe_stats"].get("maximum_toe", {}).get("raw") or ""):
                    reasons.append(f"{item['label']}: {item['toe']['note']}")
                    conditions.append(reasons[-1])
                contributions.append((quantity, value, parenthesized))
            # The chart prohibits adding a conditional gun to a supplied,
            # non-parenthesized rating in the same hex. Publish that supplied
            # loadout case, not the impossible sum of both kinds of fire.
            normal_fire = any(q > 0 and v > 0 and not p for q, v, p in contributions)
            if any(p for _, _, p in contributions):
                total_refs.append("rules-arrival-parenthesized")
                if normal_fire:
                    conditions.append("With ammunition for this loadout's non-parenthesized rating, parenthesized contributions are excluded by the chart legend; they cannot be added to that fire.")
                else:
                    conditions.append("Parenthesized ratings apply only when attacked without a friendly unit in the hex possessing ammunition and an appropriate non-parenthesized rating (3.4/chart legend). This does not authorize a voluntary attack; HQ restrictions in 3.3 also apply.")
            for quantity, value, parenthesized in contributions:
                if parenthesized and normal_fire:
                    terms.append(f"{quantity:g} × 0 (parenthesized rating excluded)")
                else:
                    terms.append(f"{quantity:g} × {value:g}")
                    total += quantity * value
            # Unknown inputs still block the whole total; no partial sum is
            # promoted merely because a different condition has been resolved.
            totals[key] = stat(total if not reasons else None, " + ".join(terms) or None, total_refs,
                               " ".join(dict.fromkeys(reasons + conditions)) if reasons or conditions else "Sum of printed arrival TOE × rating. Not-applicable combat cells contribute no intrinsic points without replacing their source cells. No combat modifiers or scenario losses applied.",
                               conditional=bool(conditions), unit="Actual AA points" if key == "anti_air" else "Raw combat points (before ÷10)")
            if (key == "close_assault_offensive" and options.get("no_assault")
                    or excluded_assault and not contributions and not reasons):
                totals[key].update(value=None, status="not_applicable", raw="May not assault")
        morale = arrival["morale_raw"]
        match = re.fullmatch(r"([+-]?\d+)(?: \((?:([+-]?\d+)|[a-z])\))?", morale or "")
        morale_value = int(match[2] or match[1]) if match else None
        quantity_values = [c["toe"]["value"] for c in components]
        missing_toe_note = " ".join(c['toe']['note'] for c in components if c['toe']['value'] is None)
        if not components:
            missing_toe_note = "No independent TOE quantity is printed for this row. " + arrival['note']
        missing_profile_note = "No characteristic ID code is printed on this OA row. " + arrival['note']
        if arrival['related_record_id']:
            missing_profile_note += " See the attachment record " + arrival['related_record_id'] + "."
        arrival_stats = {}
        if "arrival_cpa_raw" in options:
            require(options["arrival_cpa_raw"] == "0", "Unsupported reviewed arrival-CPA override")
            arrival_stats["capability_points"] = stat(0, "0", refs, arrival["note"], conditional=True, unit="Capability points")
        if related and related["kind"] == "naval_unit":
            for key in ("naval_barrage", "naval_anti_air"):
                arrival_stats[key] = copy.deepcopy(related["stats"][key])
                arrival_stats[key]["source_refs"] = list(dict.fromkeys([*arrival_stats[key]["source_refs"], *refs, "rules-arrival-naval"]))
                arrival_stats[key]["note"] = "Printed naval counter reference; actual points, not land TOE × rating. Damage reduces these strengths (30.13). " + arrival["note"]
            if "capability_points" in arrival_stats:
                arrival_stats["capability_points"]["source_refs"].append("rules-arrival-naval")
        result[record_id] = {
            "schema_version": 1, "scope": "historical_arrival_reference_not_current_state",
            "base_profile_id": profile_id, "base_profile_name": profile["name"] if profile else None,
            "base_profile_note": profile_note or (profile["note"] if profile else missing_profile_note),
            "base_stats": base, "printed_profile_code": arrival["printed_profile_code"],
            "catalog_profile_id": arrival["previous_profile_id"],
            "arrival": {
                "toe_raw": arrival["toe_raw"], "arrives_raw": arrival["arrives_raw"], "note": arrival["note"],
                "stats": arrival_stats,
                "source_refs": refs, "related_record_id": arrival["related_record_id"],
                "toe_strength": stat(sum(quantity_values) if quantity_values and all(v is not None for v in quantity_values) else None,
                                     arrival["toe_raw"], list(dict.fromkeys(ref for c in components for ref in c["toe"]["source_refs"])) or refs,
                                     options.get("calculation_note") or ("Sum of this row's printed components; excludes attachments listed in other rows." if quantity_values and all(v is not None for v in quantity_values) else missing_toe_note),
                                     conditional=any(c["toe"]["conditional"] for c in components), unit="TOE strength points"),
                "morale": stat(morale_value, morale, refs, "Parentheses give untrained morale at arrival (4.45); the outer value is trained morale." if match and match[2] else "Printed sheet/row morale." if match else "No independent numeric morale is printed for this row. " + arrival['note'],
                               unit="Morale rating"),
            },
            "components": components, "arrival_totals": totals,
            "limitations": "Historical reference only. No current CP, losses, trucks, readiness, supply or parent totals inferred. Raw combat points must be combined for an action before division/rounding; AA points are not divided by ten. Conditional ratings require their cited restrictions.",
        }
    return result
