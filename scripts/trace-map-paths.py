#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = ["Pillow==13.0.0", "numpy==2.4.6", "opencv-python-headless==5.0.0.93", "scikit-image==0.26.0"]
# ///
"""Extract display-only source centerlines, never connectivity or classifications.

Private source PNG is required. Bounded crops reuse the existing source-ink
mask, while skeletons are split at branches, gaps and crop boundaries. No gap
joining, seam joining or network links are generated. Output is separate from
the source owner's surveys and SQLite model.
"""
import argparse
from collections import defaultdict
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess

import cv2
import numpy as np
from PIL import Image
from scipy.spatial import cKDTree
from skimage.morphology import skeletonize

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "data/map/source-paths.json"
spec = importlib.util.spec_from_file_location("extract_map", ROOT / "scripts/extract-map.py")
extract = importlib.util.module_from_spec(spec)
spec.loader.exec_module(extract)


def chains(mask):
    """Pixel paths stop at branches. Suppress redundant diagonal corner edges."""
    yy, xx = np.nonzero(skeletonize(mask > 0))
    pixels = set(zip(xx.tolist(), yy.tolist()))
    adjacent = {}
    for x, y in pixels:
        adjacent[x, y] = [(x+dx, y+dy) for dx, dy in
                         ((-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1))
                         if (x+dx, y+dy) in pixels and not
                         (dx and dy and ((x+dx, y) in pixels or (x, y+dy) in pixels))]
    # Rail sleepers and occasional raster fringes produce short terminal twigs.
    # Remove only terminal chains, never bridge their former positions.
    for _ in range(2):
        remove = set()
        for start in sorted(adjacent):
            if len(adjacent[start]) != 1:
                continue
            twig, previous, current = [start], start, adjacent[start][0]
            while len(adjacent[current]) == 2 and len(twig) < 18:
                twig.append(current)
                previous, current = current, next(p for p in adjacent[current] if p != previous)
            if len(twig) < 18 and len(adjacent[current]) > 2:
                remove.update(twig)
        for p in remove:
            del adjacent[p]
        for p in adjacent:
            adjacent[p] = [q for q in adjacent[p] if q not in remove]
    used = set()
    starts = sorted(adjacent, key=lambda p: (len(adjacent[p]) == 2, p))
    result = []
    for start in starts:
        for neighbor in adjacent[start]:
            if (start, neighbor) in used:
                continue
            path = [start]
            previous, current = start, neighbor
            while True:
                used.add((previous, current))
                used.add((current, previous))
                path.append(current)
                if len(adjacent[current]) != 2 or current == start:
                    break
                following = next(p for p in adjacent[current] if p != previous)
                previous, current = current, following
            if len(path) >= 50:
                result.append(np.asarray(path, dtype=float))
    return result


def fit(raw):
    """Rounded quadratic polyline; symmetric sampled fit bound in source pixels."""
    points = cv2.approxPolyDP(raw.astype(np.float32), .6, False)[:, 0, :].astype(float)
    segments = []
    start = points[0]
    for i in range(1, len(points)-1):
        end = (points[i] + points[i+1]) / 2 if i < len(points)-2 else points[-1]
        segments.append([start.tolist(), points[i].tolist(), end.tolist()])
        start = end
    if not segments:
        segments = [[points[0].tolist(), ((points[0]+points[-1])/2).tolist(), points[-1].tolist()]]
    samples = []
    for a, b, c in np.asarray(segments):
        count = max(2, int(np.linalg.norm(b-a)+np.linalg.norm(c-b))*4)
        t = np.linspace(0, 1, count)[:, None]
        samples.extend((1-t)**2*a + 2*(1-t)*t*b + t*t*c)
    error = max(cKDTree(raw).query(samples)[0].max(), cKDTree(samples).query(raw)[0].max())
    return segments, round(float(error), 3)


def build():
    rules = json.loads((ROOT / "data/map/extraction-rules.json").read_text())
    source = ROOT / rules["source"]
    assert extract.digest(source) == rules["sha256"], "Source fingerprint mismatch"
    model = json.loads(subprocess.check_output(
        ["cargo", "run", "--quiet", "--locked", "-p", "cna-map", "--bin", "map-model"], cwd=ROOT))
    edges = {(e["hex"], e["direction"]): e for e in model["hexsides"]}
    seeds = defaultdict(list)
    for survey in model["surveys"]:
        for record in survey["records"]:
            observation = record["record"]["observation"]
            if record["kind"] != "hexside" or observation["status"] not in ("reviewed", "provisional"):
                continue
            kind = observation["value"]
            if kind in ("road", "railroad"):
                edge = edges[record["side"]["hex"], record["side"]["direction"]]
                seeds[kind].append((survey["id"], record["side"], edge))
    del model, edges
    image = Image.open(source).convert("RGB")
    output = []
    # Regional crops deliberately overlap, then keep core-owned points only.
    # Every path ends at a core boundary rather than inventing seam joins.
    for top in range(0, image.height, 768):
        for left in range(0, image.width, 768):
            box = (max(0, left-24), max(0, top-24), min(image.width, left+792), min(image.height, top+792))
            crop = np.asarray(image.crop(box))
            for kind, ink in (("road", "road"), ("railroad", "rail_track")):
                local_seeds = [(survey, side, edge) for survey, side, edge in seeds[kind]
                               if any(box[0]-3 <= x <= box[2]+3 and box[1]-3 <= y <= box[3]+3
                                      for x, y in edge["edge"])]
                if not local_seeds:
                    continue
                mask = extract.route_mask(crop, rules, ink)
                if kind == "road":
                    # Hollow road interiors are affirmative source fill, not inferred ground.
                    fill = extract.ink_masks(crop, rules)["road_fill"]
                    mask |= fill & cv2.dilate(mask, np.ones((13, 13), np.uint8))
                    mask = cv2.morphologyEx(mask, cv2.MORPH_CLOSE, np.ones((3, 3), np.uint8))
                for raw in chains(mask):
                    raw += [box[0], box[1]]
                    inside = ((raw[:, 0] >= left) & (raw[:, 0] < left+768)
                              & (raw[:, 1] >= top) & (raw[:, 1] < top+768))
                    groups = np.split(raw, np.where(inside[1:] != inside[:-1])[0]+1)
                    for points in groups:
                        if len(points) < 50 or not (left <= points[0, 0] < left+768 and top <= points[0, 1] < top+768):
                            continue
                        references = []
                        for survey, side, edge in local_seeds:
                            a, b = np.asarray(edge["edge"])
                            t = np.clip((points-a) @ (b-a) / np.sum((b-a)**2), 0, 1)
                            if np.min(np.linalg.norm(points-(a+t[:, None]*(b-a)), axis=1)) <= 3:
                                references.append({"survey_id": survey, "side": side})
                        if not references:
                            continue
                        segments, error = fit(points)
                        if error > 1.5:
                            continue
                        identity = hashlib.sha256((kind+json.dumps(points.tolist())).encode()).hexdigest()[:16]
                        output.append({"id": f"{kind}-{identity}", "depiction": kind,
                                       "review_status": "provisional", "observation_refs": references,
                                       "raw_points": points.astype(int).tolist(), "quadratics": segments,
                                       "sampled_fit_error_px": error,
                                       "bounds": [*points.min(axis=0).tolist(), *points.max(axis=0).tolist()]})
            print(f"Traced crop {left},{top}: {len(output)} paths", flush=True)
    return {"version": 1, "source_ref": rules["source"], "source_sha256": rules["sha256"],
            "units": "board pixels", "interpretation": "Display-only source centerlines; no connectivity or movement authority. Stops at gaps, branches and crop boundaries; coverage incomplete.",
            "method": "source ink skeleton with typed crossing seeds; rounded quadratics, sampled symmetric fit <=1.5px",
            "paths": output}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", required=True, help="Regenerate separate display traces from the private source")
    parser.parse_args()
    result = build()
    OUTPUT.write_text(json.dumps(result, separators=(",", ":")) + "\n")
    print(f"Wrote {len(result['paths'])} source traces to {OUTPUT}")
