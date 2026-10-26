# /// script
# requires-python = ">=3.11"
# dependencies = ["Pillow==12.0.0", "numpy==2.3.4"]
# ///
"""Bake original materials into display tiles. Never reads private source images.

The blend is illustrative, not an extraction of terrain extent. Unknown cells
are clipped out of the material masks. Coordinates come from map-model.
"""
import argparse
import hashlib
import io
import json
import math
import subprocess
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[2]
PACK = ROOT / "public/art/terrain-v1"
OUTPUT = PACK / "ground"
TILE = 512
PAD = 48  # Four times the blur radius, so adjacent delivery tiles agree.
UNKNOWN = (216, 220, 223)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def cells_from_model(model, recipes):
    materials = {}
    for survey in model["surveys"]:
        for record in survey["records"]:
            observation = record["record"]["observation"]
            if record["kind"] != "terrain" or observation["status"] not in ("reviewed", "provisional"):
                continue
            material = recipes[observation["value"]]["surface"]
            if material:
                materials.setdefault(record["hex"], set()).add(material)
    cells = []
    for cell in model["hexes"]:
        names = sorted(materials.get(cell["id"], []))
        if names:
            cells.append({"id": cell["id"], "corners": cell["corners"], "materials": names})
    return cells


def texture_region(texture, origin, size):
    """Overlap deterministic rotated patches; never mirror a repeating mountain."""
    stride = 128
    result = np.zeros((size[1], size[0], 3), dtype=np.float32)
    weights = np.zeros((size[1], size[0]), dtype=np.float32)
    for gy in range(math.floor(origin[1] / stride), math.ceil((origin[1] + size[1]) / stride) + 1):
        for gx in range(math.floor(origin[0] / stride), math.ceil((origin[0] + size[0]) / stride) + 1):
            cx, cy = gx * stride, gy * stride
            x0, y0 = max(origin[0], cx - stride), max(origin[1], cy - stride)
            x1, y1 = min(origin[0] + size[0], cx + stride), min(origin[1] + size[1], cy + stride)
            if x1 <= x0 or y1 <= y0:
                continue
            seed = ((gx * 73856093) ^ (gy * 19349663)) & 0xffffffff
            rotated = np.rot90(texture, seed % 4)
            xs, ys = np.arange(x0, x1) - cx, np.arange(y0, y1) - cy
            weight = (np.cos(ys[:, None] * np.pi / (stride * 2)) ** 2
                      * np.cos(xs[None, :] * np.pi / (stride * 2)) ** 2)
            pixels = rotated[(ys[:, None] + stride + (seed >> 4) % 64) % rotated.shape[0],
                             (xs[None, :] + stride + (seed >> 12) % 64) % rotated.shape[1]]
            region = np.s_[y0 - origin[1]:y1 - origin[1], x0 - origin[0]:x1 - origin[0]]
            result[region] += pixels * weight[:, :, None]
            weights[region] += weight
    return result / weights[:, :, None]


def render_tile(box, cells, textures):
    left, top, right, bottom = box
    origin = (left - PAD, top - PAD)
    size = (right - left + PAD * 2, bottom - top + PAD * 2)
    masks = {}
    known = Image.new("L", size)
    known_draw = ImageDraw.Draw(known)
    for cell in cells:
        points = [(x - origin[0], y - origin[1]) for x, y in cell["corners"]]
        if (max(x for x, _ in points) < 0 or min(x for x, _ in points) > size[0]
                or max(y for _, y in points) < 0 or min(y for _, y in points) > size[1]):
            continue
        known_draw.polygon(points, fill=255)
        for material in cell["materials"]:
            mask = masks.setdefault(material, Image.new("L", size))
            # Equal weights express coexistence, not measured area fractions.
            ImageDraw.Draw(mask).polygon(points, fill=round(255 / len(cell["materials"])))
    total = np.zeros((size[1], size[0]), dtype=np.float32)
    colors = np.zeros((size[1], size[0], 3), dtype=np.float32)
    for material, mask in sorted(masks.items()):
        weight = np.asarray(mask.filter(ImageFilter.GaussianBlur(12)), dtype=np.float32)
        colors += texture_region(textures[material], origin, size) * weight[:, :, None]
        total += weight
    result = np.full((size[1], size[0], 3), UNKNOWN, dtype=np.uint8)
    supported = (np.asarray(known) > 0) & (total > 0)
    result[supported] = np.clip(np.rint(colors[supported] / total[supported, None]), 0, 255).astype(np.uint8)
    return Image.fromarray(result).crop((PAD, PAD, size[0] - PAD, size[1] - PAD))


def build(check):
    exported = subprocess.check_output(
        ["cargo", "run", "--quiet", "--locked", "-p", "cna-map", "--bin", "map-model"], cwd=ROOT)
    model = json.loads(exported)
    art = json.loads((PACK / "manifest.json").read_text())
    cells = cells_from_model(model, art["recipes"]["terrain"])
    inputs = {"cells": cells, "board": art["board_pixels"],
              "surfaces": {name: asset["sha256"] for name, asset in art["assets"].items()
                           if asset["kind"] == "surface"},
              "generator": digest(Path(__file__).read_bytes())}
    fingerprint = digest(json.dumps(inputs, sort_keys=True).encode())
    if check:
        manifest = json.loads((OUTPUT / "manifest.json").read_text())
        assert manifest["inputs_sha256"] == fingerprint, "Terrain inputs changed; regenerate raster.py --write"
        for path, expected in manifest["files"].items():
            assert digest((OUTPUT / path).read_bytes()) == expected, f"Missing/corrupt terrain tile: {path}"
        print(f"Verified {len(manifest['files'])} original terrain tiles and current geometry/evidence inputs")
        return
    textures = {name: np.asarray(Image.open(PACK / asset["path"]).convert("RGB").resize(
                    (320, 320), Image.Resampling.LANCZOS))
                for name, asset in art["assets"].items() if asset["kind"] == "surface"}
    width, height = art["board_pixels"]
    board = Image.new("RGB", (width, height), UNKNOWN)
    for y in range(0, height, TILE):
        for x in range(0, width, TILE):
            box = (x, y, min(x + TILE, width), min(y + TILE, height))
            board.paste(render_tile(box, cells, textures), (x, y))
    top = math.ceil(math.log2(max(width, height) / TILE))
    manifest = {"version": 1, "inputs_sha256": fingerprint, "board_width": width,
                "board_height": height, "tile_size": TILE, "levels": [], "files": {},
                "interpretation": "Illustrative terrain presence; blended boundaries are not surveyed extents"}
    for z in range(top + 1):
        scale = 2 ** (top - z)
        w, h = math.ceil(width / scale), math.ceil(height / scale)
        image = board.resize((w, h), Image.Resampling.LANCZOS) if scale != 1 else board
        columns, rows = math.ceil(w / TILE), math.ceil(h / TILE)
        manifest["levels"].append({"z": z, "width": w, "height": h, "columns": columns, "rows": rows})
        (OUTPUT / str(z)).mkdir(parents=True, exist_ok=True)
        for y in range(rows):
            for x in range(columns):
                tile = image.crop((x * TILE, y * TILE, min((x + 1) * TILE, w), min((y + 1) * TILE, h)))
                data = io.BytesIO()
                tile.save(data, format="WEBP", lossless=True, method=4)
                path = f"{z}/{x}_{y}.webp"
                (OUTPUT / path).write_bytes(data.getvalue())
                manifest["files"][path] = digest(data.getvalue())
    (OUTPUT / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Wrote {len(manifest['files'])} original terrain tiles; no scans consumed")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    build(args.check)
