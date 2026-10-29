#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = ["Pillow==13.0.0", "numpy==2.4.6", "opencv-python-headless==5.0.0.93"]
# ///
"""Trace source water and dry-wadi outlines without assigning river/coast rules.

Usage: uv run scripts/trace-map-water.py --write
Requires the fingerprinted private scan; emits coordinates, not source imagery.
"""
import argparse
import hashlib
import json
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


def channel_mask(crop, rules, kind):
    color = np.array(rules["inks"][kind])
    ink = cv2.inRange(crop, color-3, color+3)
    # Recover only narrow black grid interruptions, not land-colored gaps.
    closed = cv2.morphologyEx(ink, cv2.MORPH_CLOSE, np.ones((5, 5), np.uint8))
    dark = (crop.max(axis=2) < 120) & (np.ptp(crop, axis=2) < 15)
    grid = np.array(rules["inks"]["grid"])
    delta = color - grid
    alpha = np.clip(((crop-grid) @ delta) / (delta @ delta), 0, 1)
    # A grid's antialiased fringe is lighter than its dark center. Omitting it
    # leaves parallel white cracks when the original print is not underneath.
    grid_blend = np.max(np.abs(crop-(grid+alpha[:, :, None]*delta)), axis=2) <= 4
    return ink | (closed & ((dark | grid_blend).astype(np.uint8)*255))


def build():
    rules = json.loads((ROOT / "data/map/extraction-rules.json").read_text())
    source = ROOT / rules["source"]
    with source.open("rb") as stream:
        assert hashlib.file_digest(stream, "sha256").hexdigest() == rules["sha256"], "Source fingerprint mismatch"
    image = Image.open(source).convert("RGB")
    outlines = []
    for kind in ("water", "wadi"):
        mask = np.zeros((image.height, image.width), dtype=np.uint8)
        for y in range(0, image.height, 256):
            top, bottom = max(0, y-3), min(image.height, y+259)
            crop = np.asarray(image.crop((0, top, image.width, bottom)))
            ink = channel_mask(crop, rules, kind)
            mask[y:min(y+256, image.height)] = ink[y-top:min(y+256, image.height)-top]
        contours, hierarchy = cv2.findContours(mask, cv2.RETR_CCOMP, cv2.CHAIN_APPROX_SIMPLE)
        for index, contour in enumerate(contours):
            if hierarchy[0, index, 3] != -1 or cv2.contourArea(contour) < 150:
                continue
            raw = [contour[:, 0, :].tolist()]
            hole = hierarchy[0, index, 2]
            while hole != -1:
                if cv2.contourArea(contours[hole]) >= 20:
                    raw.append(contours[hole][:, 0, :].tolist())
                hole = hierarchy[0, hole, 0]
            rings = [cv2.approxPolyDP(np.asarray(ring, np.int32), .75, True)[:, 0, :].tolist() for ring in raw]
            bounds = cv2.boundingRect(contour)
            identity = hashlib.sha256(json.dumps(raw).encode()).hexdigest()[:16]
            outlines.append({"id": f"{kind}-{identity}", "kind": kind, "raw_rings": raw, "rings": rings,
                             "bounds": [bounds[0], bounds[1], bounds[0]+bounds[2], bounds[1]+bounds[3]]})
    return {"version": 1, "source_ref": rules["source"], "source_sha256": rules["sha256"],
            "review_status": "provisional", "units": "board pixels", "simplification_tolerance_px": .75,
            "method": "separate water and dry-wadi ink contours; narrow dark/grid-blend interruptions repaired; pale land gaps and larger overprints retained",
            "interpretation": "Source ink outlines, not Major/Minor River, sea, coast or movement classifications. Wadi ink stays separate from water. Original channel widths retained; holes preserved; no gap joining across land.",
            "outlines": outlines}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true", required=True)
    parser.parse_args()
    result = build()
    (ROOT / "data/map/source-water.json").write_text(json.dumps(result, separators=(",", ":")) + "\n")
    print(f"Wrote {len(result['outlines'])} water outlines with original widths and holes")
