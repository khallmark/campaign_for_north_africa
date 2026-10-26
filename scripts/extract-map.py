#!/usr/bin/env python3
"""Survey every source-grid cell and internal side; retain ambiguous ink evidence.

Requires the private original PNG, numpy, Pillow and OpenCV. Outputs measurements,
not artwork. No absent ink is promoted to confirmed feature absence. Memory is
bounded by one RGB source and small cell/side crops, not full-image label arrays.
"""
import argparse
from collections import Counter, defaultdict
import hashlib
import json
from pathlib import Path
import subprocess

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
RULES = ROOT / "data/map/extraction-rules.json"
LEDGER = ROOT / "data/map/examination.json"
SURVEYS = ROOT / "data/map/extracted-surveys.json"
FAMILIES = ("surface", "terrain", "transport", "hexside", "locations")
BACKGROUNDS = ("background", "road_fill", "rough", "salt_marsh", "mountain", "delta",
               "desert", "slope_ridge", "water", "gravel", "vegetation", "swamp")


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def encode(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def crop_polygon(image, points, inset=0):
    points = np.array(points, dtype=float)
    if inset:
        points = points.mean(axis=0) + (points - points.mean(axis=0)) * inset
    x0, y0 = np.floor(points.min(axis=0)).astype(int) - 1
    x1, y1 = np.ceil(points.max(axis=0)).astype(int) + 2
    crop = np.full((y1-y0, x1-x0, 3), 0, dtype=np.uint8)
    left, top, right, bottom = max(x0, 0), max(y0, 0), min(x1, image.width), min(y1, image.height)
    if left < right and top < bottom:
        crop[top-y0:bottom-y0, left-x0:right-x0] = image.crop((left, top, right, bottom))
    mask = np.zeros(crop.shape[:2], dtype=np.uint8)
    cv2.fillConvexPoly(mask, np.round(points - [x0, y0]).astype(np.int32), 1)
    return crop, mask.astype(bool), [int(x0), int(y0), int(x1-x0), int(y1-y0)]


def translucent_foreground(rules, ink):
    opacity = rules[ink+"_opacity"]
    return (np.array(rules["inks"][ink]) -
            (1-opacity) * np.array(rules["inks"]["background"])) / opacity


def ink_masks(crop, rules):
    tolerance = rules["tolerance"]
    masks = {name: cv2.inRange(crop, np.maximum(np.array(color)-tolerance, 0),
                             np.minimum(np.array(color)+tolerance, 255))
             for name, color in rules["inks"].items()}
    # Translucent ink changes color over terrain; white-ground RGB alone loses
    # routes and barred lines. These fitted swatches remain provisional evidence.
    for ink in ("rail_track", "blue_barred"):
        opacity = rules[ink+"_opacity"]
        for background in BACKGROUNDS:
            color = opacity * translucent_foreground(rules, ink) + (1-opacity) * np.array(rules["inks"][background])
            masks[ink] |= cv2.inRange(crop, color-tolerance, color+tolerance)
    return masks


def route_mask(crop, rules, ink):
    """Recover antialiased strokes against the inspected background palette."""
    foreground = translucent_foreground(rules, ink) if ink == "rail_track" else np.array(rules["inks"][ink], dtype=float)
    result = np.zeros(crop.shape[:2], dtype=bool)
    for name in BACKGROUNDS:
        delta = np.array(rules["inks"][name]) - foreground
        alpha = np.clip(((crop-foreground) @ delta) / (delta @ delta), 0, 1)
        fitted = foreground + alpha[:, :, None] * delta
        result |= ((np.max(np.abs(crop-fitted), axis=2) < (8 if ink == "road" else 2))
                   & (alpha < (.85 if ink == "road" else .65)))
    # Some brown terrain swatches also lie on a road/background blend line.
    # Their affirmative palette match takes precedence over that interpolation.
    exact = ink_masks(crop, rules)
    for name, mask in exact.items():
        if name != ink:
            result &= mask == 0
    if ink == "road":
        # Wider tolerance handles road-edge antialiasing against both the fill
        # and surrounding terrain, but only near affirmative brown source ink.
        result &= cv2.dilate(exact[ink], np.ones((7, 7), np.uint8)) > 0
    mask = result.astype(np.uint8) * 255
    # The black grid sometimes overwrites a route with mixed dark-grey pixels.
    # Bridge only short dark interruptions, never light gaps between unfinished
    # crosses/dashes. Unconditional closing conflates those two situations.
    closed = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8))
    dark = (crop.max(axis=2) < 120) & (np.ptp(crop, axis=2) < 15)
    mask |= closed & (dark.astype(np.uint8) * 255)
    if ink == "rail_track":
        # Escarpment antialiasing also fits gray ink and its jagged edge can
        # mimic ties. Require a solid escarpment patch before excluding its
        # fringe: isolated antialiased route pixels also share this swatch.
        solid = cv2.morphologyEx(exact["escarpment"], cv2.MORPH_OPEN, np.ones((3, 3), np.uint8))
        mask[cv2.dilate(solid, np.ones((5, 5), np.uint8)) > 0] = 0
    return mask


def point_candidates(image, cell, rules, templates):
    crop, _, bounds = crop_polygon(image, cell["corners"], inset=1.9)
    masks = ink_masks(crop, rules)
    mask = masks["blue_symbols_text"]
    n, _, stats, centers = cv2.connectedComponentsWithStats(mask)
    polygon = np.array(cell["corners"], np.float32)
    points = []
    for index in range(1, n):
        _, _, width, height, area = stats[index]
        x, y = centers[index] + bounds[:2]
        if (8 <= width <= 18 and 8 <= height <= 18 and .75 <= width/height <= 1.33
                and area / (width*height) >= .70 and area >= 60
                and cv2.pointPolygonTest(polygon, (float(x), float(y)), False) >= 0):
            points.append({"symbol": "village_bir_dot", "center": [round(float(x), 2), round(float(y), 2)],
                           "status": "unresolved", "reason": "Village/Bir glyph detected; label association and category need review"})
    for spec, template in templates:
        scores = cv2.matchTemplate(masks[spec["ink"]], template, cv2.TM_CCOEFF_NORMED)
        yy, xx = np.where(scores >= spec["minimum_score"])
        accepted = []
        for score, x, y in sorted(((float(scores[y, x]), int(x), int(y)) for y, x in zip(yy, xx)), reverse=True):
            position = np.array([x + template.shape[1]/2 + bounds[0], y + template.shape[0]/2 + bounds[1]])
            if cv2.pointPolygonTest(polygon, tuple(position), False) < 0 or any(np.linalg.norm(position-p) < 20 for p in accepted):
                continue
            accepted.append(position)
            points.append({"symbol": spec["symbol"], "center": position.tolist(), "score": round(score, 5),
                           "template_crop": spec["crop"], "status": "provisional" if spec["symbol"] == "oasis" else "unresolved",
                           "reason": "Source template match; port operational association and efficiency are not inferred"})
    return points


def examine_cell(image, cell, rules, templates=()):
    crop, polygon, bounds = crop_polygon(image, cell["corners"])
    masks = ink_masks(crop, rules)
    area = int(polygon.sum())
    counts = {name: int(np.count_nonzero(mask[polygon])) for name, mask in masks.items()}
    # Full-cell coverage keeps partial and mixed patches visible in the ledger.
    terrain = [name for name in ("rough", "delta") if counts[name] / area >= .08]
    # Mountain ink also occurs in narrow slope bands. Require a broad solid core.
    if np.count_nonzero(cv2.erode(masks["mountain"], np.ones((21, 21), np.uint8))[polygon]) >= 30:
        terrain.append("mountain")
    if counts["gravel"] >= 40:
        terrain.append("gravel")
    # Desert shares yellow with salt-marsh strokes, so require a substantial fill.
    if counts["desert"] / area >= .20:
        terrain.append("desert")
    if counts["salt_marsh"] / area >= .04 and counts["desert"] >= 8:
        terrain.append("salt_marsh")
    for ink, value in (("vegetation", "heavy_vegetation"), ("swamp", "swamp")):
        if counts[ink] >= 40:
            terrain.append(value)
    water = counts["water"] / area
    land = sum(counts[k] for k in ("background", "rough", "mountain", "delta", "desert", "salt_marsh", "gravel", "slope_ridge", "escarpment", "wadi")) / area
    surface = "water" if water >= .90 else "mixed_water_land" if water >= .04 and land >= .04 else "land" if land >= .45 and water < .04 else "unresolved"
    # A positive blank-swatch comparison, not a fallback for unrecognized cells.
    inner, inside, _ = crop_polygon(image, cell["corners"], inset=.80)
    blank = ink_masks(inner, rules)["background"]
    # Transportation/grid antialiasing is not terrain. Its palette calibration
    # must not change an otherwise affirmative blank terrain-swatch comparison.
    special = sum(counts[k] for k in ("water", "rough", "mountain", "delta", "desert",
                  "salt_marsh", "gravel", "slope_ridge", "escarpment", "wadi", "vegetation", "swamp"))
    if surface == "land" and special < 5 and np.count_nonzero(blank[inside]) / inside.sum() >= .995:
        terrain.append("clear")
    points = point_candidates(image, cell, rules, templates)
    states = {
        "surface": "detected" if surface != "unresolved" else "unresolved",
        "terrain": "detected" if terrain else "unresolved",
        "transport": "candidate" if counts["road"] + counts["rail_track"] >= 10 else "not_detected",
        "hexside": "candidate" if sum(counts[k] for k in ("wadi", "slope_ridge", "escarpment", "blue_barred")) >= 10 else "not_detected",
        # Ink includes lettering. Its presence is not a settlement assignment.
        "locations": "candidate" if counts["blue_symbols_text"] >= 10 or points else "not_detected",
    }
    return {"hex": cell["id"], "section": cell["section"], "crop": bounds,
            "pixels": area, "ink_pixels": counts, "surface": surface,
            "terrain": terrain, "points": points, "families": states}


def dashed_route_context(mask, bounds, a, b):
    """Recognize repeated narrow dashes across a grid-contaminated crossing.

    The strip alone can contain only a grid-merged dash and two clipped dashes.
    Use the surrounding source pixels, not graph endpoints, to recover it.
    """
    along = (b-a) / np.linalg.norm(b-a)
    normal = np.array([-along[1], along[0]])
    y, x = np.indices(mask.shape)
    world = np.stack((x+bounds[0]-a[0], y+bounds[1]-a[1]), axis=-1)
    across = world @ normal
    clean = mask.copy()
    # Exclude the calibrated grid band from dash-shape fitting only. The ink
    # profile below still checks the unmodified source mask across this band.
    clean[np.abs(across) < 3] = 0
    n, labels, stats, _ = cv2.connectedComponentsWithStats(clean)
    fragments = []
    for component in range(1, n):
        if not 12 <= stats[component, cv2.CC_STAT_AREA] <= 85:
            continue
        points = world[labels == component]
        values, axes = np.linalg.eigh(np.cov(points.T))
        axis = axes[:, -1]
        length = np.ptp(points @ axis)
        width = np.ptp(points @ axes[:, 0])
        if 8 <= length <= 24 and width < 3.5 and values[-1] > 12 * values[0]:
            fragments.append((points.mean(axis=0), axis))
    for i, (p, p_axis) in enumerate(fragments):
        for q, q_axis in fragments[i+1:]:
            pn, qn = p @ normal, q @ normal
            distance = np.linalg.norm(q-p)
            if pn * qn >= 0 or not 12 < distance < 65 or abs(pn-qn) < 10:
                continue
            axis = (q-p) / distance
            if min(abs(axis @ p_axis), abs(axis @ q_axis)) < .97:
                continue
            crossing = p + (q-p) * (-pn/(qn-pn))
            t = (crossing @ along) / np.linalg.norm(b-a)
            if not .12 < t < .88:
                continue
            transverse = np.array([-axis[1], axis[0]])
            relative = world-p
            u, v = relative @ axis, relative @ transverse
            corridor = (u >= -8) & (u <= distance+8) & (np.abs(v) < 5)
            ink = (mask > 0) & corridor
            bins = np.floor(u[ink]+8).astype(int)
            occupied = np.bincount(bins, minlength=int(distance)+17) > 0
            profile = "".join("1" if value else "0" for value in occupied)
            runs = [len(run) for run in profile.split("0") if run]
            gaps = [len(run) for run in profile.strip("0").split("1") if run]
            # Three observed dashes with short gaps, not arbitrary gap filling.
            # Continuous spines and isolated unfinished crosses fail this test.
            if sum(length >= 7 for length in runs) < 3 or not gaps or max(gaps) > 9:
                continue
            wide = ink & (np.abs(v) > 2.6) & (np.abs(across) > 3)
            if np.count_nonzero(wide) > 8:
                continue
            return [{"span": round(float(distance), 2), "t": round(float(t), 3),
                     "runs": runs, "gaps": gaps}]
    return []


def continuous_route_context(image, a, b, rules, ink):
    """Trace long source strokes across a side, including curves and termini.

    Continuity length is measured along the route, not normal to the hexside.
    A shallow crossing or a terminus need not extend 60px on both sides.
    """
    along = (b-a) / np.linalg.norm(b-a)
    normal = np.array([-along[1], along[0]])
    midpoint = (a+b)/2
    crop, _, bounds = crop_polygon(image, [midpoint+offset for offset in (
        [-120, -120], [120, -120], [120, 120], [-120, 120])])
    mask = route_mask(crop, rules, ink)
    n, labels, stats, _ = cv2.connectedComponentsWithStats(mask)
    evidence = []
    for component in range(1, n):
        if stats[component, cv2.CC_STAT_AREA] < 80:
            continue
        y, x = np.nonzero(labels == component)
        world = np.column_stack((x+bounds[0], y+bounds[1])) - a
        across = world @ normal
        t = world @ along / np.linalg.norm(b-a)
        if not (across.min() < -10 and across.max() > 10
                and np.any((np.abs(across) < 2) & (t > .12) & (t < .88))):
            continue
        _, axes = np.linalg.eigh(np.cov(world.T))
        along_route = world @ axes[:, -1]
        length = float(np.ptp(along_route))
        if length < 90:
            continue
        positions = np.floor(along_route).astype(int)
        transverse = world @ axes[:, 0]
        widths = [np.ptp(transverse[positions == p]) if np.count_nonzero(positions == p) > 1 else 0
                  for p in range(positions.min(), positions.max()+1)]
        peaks = sum(bool(run) for run in "".join("1" if w >= 6 else "0" for w in widths).split("0"))
        if ink == "road" or peaks >= 4:
            evidence.append({"length": round(length, 2), "tie_peaks": peaks})
    return {"crop": bounds, "strokes": evidence,
            "dashes": dashed_route_context(mask, bounds, a, b) if ink == "rail_track" else []}


def examine_side(image, side, rules):
    a, b = np.array(side["edge"])
    along = (b-a) / np.linalg.norm(b-a)
    normal = np.array([-along[1], along[0]])
    points = [a-normal*30, b-normal*30, b+normal*30, a+normal*30]
    crop, polygon, bounds = crop_polygon(image, points)
    masks = ink_masks(crop, rules)
    findings = []
    for ink in ("road", "rail_track", "wadi", "slope_ridge", "escarpment", "water", "blue_barred"):
        mask = masks[ink].copy()
        mask[~polygon] = 0
        count = int(np.count_nonzero(mask))
        if count < 8:
            continue
        # Reconstruct antialiasing and dark grid occlusion, never light gaps
        # between unfinished transportation strokes.
        if ink in ("road", "rail_track"):
            mask = route_mask(crop, rules, ink)
            mask[~polygon] = 0
        contours, hierarchy = cv2.findContours(mask, cv2.RETR_CCOMP, cv2.CHAIN_APPROX_SIMPLE)
        capsules = sum(1 for index, contour in enumerate(contours)
                       if hierarchy[0][index][3] >= 0 and cv2.contourArea(contour) >= 3) if hierarchy is not None else 0
        joined = mask if ink in ("road", "rail_track") else cv2.morphologyEx(mask, cv2.MORPH_CLOSE, np.ones((3, 3), np.uint8))
        n, labels, stats, _ = cv2.connectedComponentsWithStats(joined)
        crossing = False
        corner = False
        continuous = False
        tied = False
        fragments = []
        for component in range(1, n):
            if stats[component, cv2.CC_STAT_AREA] < 12:
                continue
            y, x = np.nonzero(labels == component)
            world = np.column_stack((x + bounds[0], y + bounds[1])) - a
            across = world @ normal
            t = world @ along / np.linalg.norm(b-a)
            covariance, axes = np.linalg.eigh(np.cov(world.T))
            aspect = float(np.sqrt(covariance[-1] / max(covariance[0], .01)))
            axis = axes[:, -1]
            fragments.append((world.mean(axis=0), aspect, len(x), axis))
            if across.min() < -11 and across.max() > 11:
                near = t[np.abs(across) < 3]
                if len(near):
                    if np.median(near) < .12 or np.median(near) > .88:
                        corner = True
                    else:
                        crossing = True
                        raw = world[mask[y, x] > 0]
                        positions = np.floor(raw @ axis).astype(int)
                        occupied = np.bincount(positions - positions.min()) > 0
                        gaps = [len(run) for run in "".join("1" if v else "0" for v in occupied).split("1")]
                        uninterrupted = len(occupied) >= 48 and max(gaps, default=0) == 0
                        continuous |= uninterrupted
                        transverse = raw @ axes[:, 0]
                        widths = [np.ptp(transverse[positions == p]) if np.count_nonzero(positions == p) > 1 else 0
                                  for p in range(positions.min(), positions.max()+1)]
                        peaks = sum(1 for run in "".join("1" if w >= 6 else "0" for w in widths).split("0") if run)
                        tied |= uninterrupted and peaks >= 3
        # Wadi/ridge/escarpment run ALONG sides; a transverse test would be wrong.
        y, x = np.nonzero(mask)
        world = np.column_stack((x + bounds[0], y + bounds[1])) - a
        t = world @ along / np.linalg.norm(b-a)
        across = world @ normal
        side_band = np.count_nonzero((t > .15) & (t < .85) & (np.abs(across) < 10)) >= 80
        if ink == "blue_barred":
            # Lettering/city blocks can share blue ink. Require repeated ink
            # along the central side, not merely a dot or label near a corner.
            bins = np.floor(t * 8).astype(int)
            occupied = {int(v) for v in bins[np.abs(across) < 10] if 1 <= v <= 6}
            side_band = (np.count_nonzero((t > .15) & (t < .85) & (np.abs(across) < 10)) >= 30
                         and len(occupied) >= 4)
        context = continuous_route_context(image, a, b, rules, ink) if ink in ("road", "rail_track") else None
        # Repeated source strokes: single narrow dashes versus cross-tied marks.
        # Keep unfinished/irregular or mixed patterns unresolved.
        patterns = set()
        if ink == "rail_track":
            for i, (p, aspect, mass, axis) in enumerate(fragments):
                for q, other_aspect, other_mass, other_axis in fragments[i+1:]:
                    distance = np.linalg.norm(q-p)
                    pn, qn = p @ normal, q @ normal
                    if 10 < distance < 32 and pn*qn < 0 and abs(pn-qn) > 8:
                        crossing_point = p + (q-p) * (-pn/(qn-pn))
                        t_cross = (crossing_point @ along) / np.linalg.norm(b-a)
                        if .12 < t_cross < .88:
                            direction = (q-p) / distance
                            if (aspect > 3 and other_aspect > 3 and mass < 80 and other_mass < 80
                                    and abs(axis @ direction) > .96 and abs(other_axis @ direction) > .96):
                                patterns.add("track")
            if context["dashes"]:
                patterns.add("track")
            if context["strokes"]:
                patterns.add("railroad")
        value = "road" if ink == "road" and context["strokes"] else "wadi" if ink == "wadi" and side_band else next(iter(patterns)) if len(patterns) == 1 else None
        findings.append({"ink": ink, "pixels": count, "crossing": crossing,
                         "corner": corner, "continuous": bool(continuous), "ties": bool(tied),
                         "capsules": capsules, "side_band": bool(side_band), "context": context, "value": value})
    return {"hex": side["hex"], "direction": side["direction"], "other": side["other"],
            "crop": bounds, "findings": findings}


def generate():
    rules = json.loads(RULES.read_text())
    sources = {rules["source"]: rules["sha256"], rules["legend"]: rules["legend_sha256"]}
    for path, expected in sources.items():
        if digest(ROOT / path) != expected:
            raise ValueError(f"source fingerprint changed: {path}; do not silently update pins")
    model = json.loads(subprocess.check_output(
        ["cargo", "run", "--quiet", "--locked", "-p", "cna-map", "--bin", "map-model", "--", "--geometry-only"], cwd=ROOT))
    with Image.open(ROOT / rules["source"]) as source:
        image = source.convert("RGB")
    templates = []
    for spec in rules["templates"]:
        x, y, w, h = spec["crop"]
        templates.append((spec, ink_masks(np.array(image.crop((x, y, x+w, y+h))), rules)[spec["ink"]]))
    cells = [examine_cell(image, cell, rules, templates) for cell in model["hexes"]]
    sides = [examine_side(image, side, rules) for side in model["hexsides"]]
    surveys = []
    groups = defaultdict(list)
    by_hex = {c["hex"]: c for c in cells}
    labels = json.loads((ROOT / "data/map/location-labels.json").read_text())
    for hex_id, label in labels["labels"].items():
        points = [p for p in by_hex[hex_id]["points"] if p["symbol"] == "village_bir_dot"]
        if len(points) != 1:
            raise ValueError(f"label association requires one detected dot: {hex_id}")
        point = points[0]
        point["label"] = label
        x, y = point["center"]
        point["label_crop"] = [int(x)-220, int(y)-90, 440, 150]
        surveys.append({"id": f"label-{hex_id}", "label": label, "notes": labels["notes"],
            "provenance": {"source_refs": [rules["source"]+"#xywh="+",".join(map(str, point["label_crop"])),
                                             rules["legend"]+"#legend/Village-Bir"],
                           "reviewer": labels["reviewer"], "date": labels["date"]},
            "coverage": {"kind": "point", "hexes": [hex_id], "observation": {
                "status": "unresolved", "source_label": label+"; Village/Bir glyph, operational category unresolved"}}})
    for cell in cells:
        for terrain in cell["terrain"]:
            groups[(cell["section"], "terrain", terrain)].append(cell["hex"])
        for symbol in sorted({point["symbol"] for point in cell["points"]}):
            if symbol == "village_bir_dot" and cell["hex"] in labels["labels"]:
                continue
            groups[(cell["section"], "point", symbol)].append(cell["hex"])
    for side in sides:
        for finding in side["findings"]:
            section = by_hex[side["hex"]]["section"]
            if finding["value"]:
                groups[(section, "hexside", finding["value"])].append({"hex": side["hex"], "direction": side["direction"]})
            elif finding["ink"] in ("road", "rail_track") or (finding["side_band"] and finding["ink"] in ("slope_ridge", "escarpment", "blue_barred")):
                groups[(section, "hexside", "unresolved_"+finding["ink"])].append({"hex": side["hex"], "direction": side["direction"]})
            elif finding["ink"] == "water" and finding["side_band"] and any(by_hex[h]["surface"] != "water" for h in (side["hex"], side["other"])):
                groups[(section, "hexside", "unresolved_river_coast")].append({"hex": side["hex"], "direction": side["direction"]})
    for (section, kind, value), locations in sorted(groups.items()):
        if kind == "point":
            observation = ({"status": "provisional", "value": {"type": "oasis"}} if value == "oasis" else
                           {"status": "unresolved", "source_label": value+" glyph; name, category or operational association unresolved"})
        elif value.startswith("unresolved_"):
            observation = {"status": "unresolved", "source_label": value.removeprefix("unresolved_")+" ink; class, construction state, crossing or orientation unresolved"}
        else:
            observation = {"status": "provisional", "value": value}
        surveys.append({"id": f"ink-{section}-{kind}-{value}", "label": f"{section} source-ink {value}",
            "notes": "Automated source-ink evidence, not human-reviewed. Per-location pixel counts and original-image crop coordinates are in data/map/examination.json. Partial patches retained; absence is not inferred. Networks are section-local observed crossings, not legal movement or junction authority.",
            "provenance": {"source_refs": [rules["source"]+f"#examination/{section}/{kind}/{value}", rules["legend"]+"#legend"],
                           "reviewer": "Amp automated ink survey; human review pending", "date": "2026-10-25"},
            "coverage": {"kind": kind, "sides" if kind == "hexside" else "hexes": locations,
                         "observation": observation}})
    coverage = defaultdict(Counter)
    for cell in cells:
        for family, state in cell["families"].items():
            coverage[cell["section"]][family+":"+state] += 1
    fingerprints = {path: digest(ROOT / path) for path in (
        "scripts/extract-map.py", "requirements-map.txt", "data/map/extraction-rules.json", "data/map/location-labels.json", "crates/cna-map/src/geometry.rs",
        "crates/cna-map/src/calibration.rs", "crates/cna-map/src/hex.rs", "public/data/map/sources.json")}
    ledger = {"version": 1, "method": rules["method"], "sources": sources,
              "input_fingerprints": fingerprints,
              "geometry_sha256": hashlib.sha256(encode({k: model[k] for k in ("hexes", "hexsides")}).encode()).hexdigest(),
              "status_definitions": {
                  "detected": "Provisional measured ink interpretation; other features may be missing",
                  "candidate": "Ink found; symbol/geometry interpretation unresolved",
                  "not_detected": "Examined by this detector, not confirmed absent; unsupported symbols may remain",
                  "unresolved": "Raster examined; no supported interpretation, never default Clear"},
              "limitations": rules["notes"], "coverage": coverage, "cells": cells, "sides": sides}
    extracted = {"version": 1, "sources": sources, "surveys": surveys,
                 "examinations": {c["hex"]: {k: c[k] for k in ("crop", "surface", "families")} for c in cells}}
    ledger["extracted_sha256"] = hashlib.sha256((encode(extracted)+"\n").encode()).hexdigest()
    return ledger, extracted


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true", help="regenerate measurements and provisional surveys")
    mode.add_argument("--check", action="store_true", help="re-examine original pixels and compare outputs (default)")
    args = parser.parse_args()
    ledger, surveys = generate()
    for path, value in ((LEDGER, ledger), (SURVEYS, surveys)):
        content = encode(value) + "\n"
        if args.write:
            path.write_text(content)
        elif not path.exists() or path.read_text() != content:
            raise ValueError(f"stale {path.relative_to(ROOT)}; review inputs before --write")
    print(json.dumps({"cells": len(ledger["cells"]), "sides": len(ledger["sides"]), "coverage": ledger["coverage"]}, indent=2))


if __name__ == "__main__":
    main()
