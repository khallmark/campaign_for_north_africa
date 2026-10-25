# /// script
# requires-python = ">=3.11"
# dependencies = ["Pillow==12.0.0"]
# ///
"""Extract original Painter masters and verify the layered art contract."""

import argparse
import hashlib
import io
import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
PACK = ROOT / "public/art/terrain-v1"
SURFACES = "clear gravel salt_marsh heavy_vegetation rough mountain delta desert swamp".split()
TRANSPORT = "road road_unfinished railroad railroad_unfinished major_river minor_river".split()
FEATURES = "track ridge slope escarpment wadi coast pipeline sea unknown".split()
POINTS = (
    "major_city village bir oasis well airfield landing_strip flying_boat_basin "
    "port fortification_level_one fortification_level_two fortification_level_three "
    "minefield friendly_badge enemy_badge village_bir_oasis"
).split()
SHEETS = {"surfaces": (3, SURFACES), "transport": (2, TRANSPORT),
          "features": (3, FEATURES), "points": (4, POINTS)}


def png_bytes(image):
    output = io.BytesIO()
    image.save(output, format="PNG", optimize=True)
    return output.getvalue()


def extract(master, columns, count, index, opaque):
    rows = count // columns
    col, row = index % columns, index // columns
    box = [round(col * master.width / columns), round(row * master.height / rows),
           round((col + 1) * master.width / columns), round((row + 1) * master.height / rows)]
    if opaque:
        # Exclude painted transitions between neighboring atlas swatches.
        box = [box[0] + 8, box[1] + 8, box[2] - 8, box[3] - 8]
    image = master.crop(box).convert("RGBA")
    if not opaque:
        # Painter leaves near-invisible alpha dust across some empty cells.
        image.putalpha(image.getchannel("A").point(lambda value: 0 if value <= 8 else value))
        bounds = image.getchannel("A").getbbox()
        if bounds is None:
            raise ValueError(f"Empty sprite at cell {index}")
        box = [box[0] + bounds[0], box[1] + bounds[1],
               box[0] + bounds[2], box[1] + bounds[3]]
        image = image.crop(bounds)
        padded = Image.new("RGBA", (image.width + 16, image.height + 16))
        padded.paste(image, (8, 8))
        image = padded
    return image, box


def strip_anchors(image):
    """Visual attachment points, not evidence-derived geographic centerlines."""
    alpha = image.getchannel("A")
    anchors = []
    for fraction in (0.05, 0.95):
        x = round(8 + (image.width - 17) * fraction)
        weights = [sum(alpha.getpixel((xx, y)) for xx in range(x - 2, x + 3))
                   for y in range(image.height)]
        mass = sum(weights)
        if not mass:
            raise ValueError("Anchor misses artwork")
        y = sum(y * weight for y, weight in enumerate(weights)) / mass
        anchors.append([round(x / image.width, 6), round(y / image.height, 6)])
    return anchors


def recipes():
    terrain = {name: {"surface": name, "stamps": []} for name in SURFACES}
    for name in ["major_city", "village_bir_oasis", "fortification_level_one",
                 "fortification_level_two", "fortification_level_three"]:
        terrain[name] = {"surface": None, "stamps": [name]}
    for relation in ("friendly", "enemy"):
        terrain[f"{relation}_minefield"] = {
            "surface": None, "stamps": ["minefield", f"{relation}_badge"],
            "requires": "viewer-relative simulation state; not permanent geography",
        }
    sides = {name: {"asset": name} for name in
             "railroad road track ridge wadi major_river minor_river coast sea pipeline".split()}
    for feature in ("slope", "escarpment"):
        for direction in ("up", "down"):
            sides[f"{direction}_{feature}"] = {
                "asset": feature,
                "high_side": "toward_neighbor" if direction == "up" else "toward_canonical_hex",
            }
    points = {name: {"asset": name} for name in POINTS[:9]}
    points["port"]["requires"] = "retain efficiency_ref; art implies no capacity"
    points["fortification"] = {"level": {
        level: f"fortification_level_{level}" for level in ("one", "two", "three")}}
    return {"terrain": terrain, "hexside": sides, "point": points}


def validate(manifest, images):
    schema = json.loads((ROOT / "docs/map/map-v1.schema.json").read_text())["$defs"]
    for category in ("terrain", "hexside"):
        expected = {item["const"] for item in schema[category]["oneOf"]}
        if set(manifest["recipes"][category]) != expected:
            raise ValueError(f"{category} vocabulary changed; update art coverage")
    point_names = set()
    for item in schema["point"]["oneOf"]:
        kind = item["properties"]["type"]
        point_names.update(kind["enum"] if "enum" in kind else [kind["const"]])
    if set(manifest["recipes"]["point"]) != point_names:
        raise ValueError("Point vocabulary changed; update art coverage")
    for name, image in images.items():
        alpha = image.getchannel("A")
        low, high = alpha.getextrema()
        if high < 128 or (name in SURFACES and low != 255):
            raise ValueError(f"{name}: invalid surface/foreground alpha")
        if name not in SURFACES:
            if low != 0:
                raise ValueError(f"{name}: missing transparency")
            for edge in [(0, 0, image.width, 1), (0, image.height - 1, image.width, image.height),
                         (0, 0, 1, image.height), (image.width - 1, 0, image.width, image.height)]:
                if alpha.crop(edge).getextrema()[1]:
                    raise ValueError(f"{name}: nontransparent padding")
    for finished in ("road", "railroad"):
        image = images[f"{finished}_unfinished"].getchannel("A")
        # Construction must contain an actual interior gap, not only a caption.
        occupied = [image.crop((x, 0, x + 1, image.height)).getextrema()[1] > 32
                    for x in range(image.width // 5, image.width * 4 // 5)]
        if all(occupied):
            raise ValueError(f"{finished}: construction sprite has no visible gap")


def build(check):
    provenance = json.loads((PACK / "provenance.json").read_text())
    manifest = {
        "version": 1, "units": "normalized image coordinates; x right, y down",
        "geometry_authority": "crates/cna-map/src/geometry.rs and calibration.rs",
        "board_pixels": [14310, 4632], "per_hex_art_contract": False,
        "rendering": "illustrative decals; not seamless textures or geographic centerlines",
        "unknown": {"fill": "#d8dcdf", "asset": "unknown", "label": "Unknown"},
        "construction_variants": {"road": "road_unfinished", "railroad": "railroad_unfinished"},
        "construction_policy": "explicit construction evidence only; provisional does not mean unfinished",
        "recipes": recipes(), "assets": {}, "masters": {},
    }
    outputs, images = {}, {}
    for sheet, (columns, names) in SHEETS.items():
        path = PACK / "masters" / f"{sheet}.png"
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != provenance["masters"][sheet]["sha256"]:
            raise ValueError(f"{sheet}: master hash differs from recorded Painter output")
        master = Image.open(path)
        manifest["masters"][sheet] = {"path": f"masters/{sheet}.png", "sha256": digest,
                                       "size": list(master.size)}
        for index, name in enumerate(names):
            surface = sheet == "surfaces"
            strip = sheet == "transport" or (sheet == "features" and name not in ("sea", "unknown"))
            image, source_box = extract(master, columns, len(names), index, surface)
            relative = f"{'surfaces' if surface else 'overlays'}/{name}.png"
            encoded = png_bytes(image)
            outputs[relative] = encoded
            images[name] = image
            entry = {"path": relative, "size": list(image.size), "pivot": [0.5, 0.5],
                     "kind": "surface" if surface else "strip" if strip else "stamp",
                     "alpha": not surface, "master": sheet, "source_box": source_box,
                     "padding": 0 if surface else 8, "sha256": hashlib.sha256(encoded).hexdigest()}
            if strip:
                entry["anchors"] = strip_anchors(image)
            if name in ("slope", "escarpment"):
                entry["high_side_normal"] = [0, -1]
            if name == "coast":
                entry["water_side_normal"] = [0, -1]
            manifest["assets"][name] = entry
    validate(manifest, images)
    outputs["manifest.json"] = (json.dumps(manifest, indent=2) + "\n").encode()
    for relative, expected in outputs.items():
        path = PACK / relative
        if check:
            if not path.exists() or path.read_bytes() != expected:
                raise ValueError(f"Stale or missing {path}; run build.py --write")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(expected)
    print(f"{'Verified' if check else 'Built'} {len(images)} assets: 16 terrain, 14 hexside, "
          "10 point kinds; alpha, construction gaps, provenance and deterministic output OK")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, epilog=(
        "Examples: uv run scripts/terrain-art/build.py --write\n"
        "          uv run scripts/terrain-art/build.py --check"),
        formatter_class=argparse.RawDescriptionHelpFormatter)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="Regenerate assets and manifest from checked-in masters")
    mode.add_argument("--check", action="store_true", help="Read-only coverage, alpha, hash and output verification")
    args = parser.parse_args()
    try:
        build(args.check)
    except (ValueError, OSError) as error:
        parser.exit(1, f"terrain-art: {error}\n")
