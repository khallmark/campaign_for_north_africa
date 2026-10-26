# /// script
# requires-python = ">=3.11"
# dependencies = ["Pillow==12.0.0"]
# ///
"""Render synthetic art samples using exported map geometry, never source scans."""

import argparse
import json
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFont, ImageOps

from build import PACK, SURFACES


def text(image, xy, value, size=20, color="#26323a"):
    ImageDraw.Draw(image).text(xy, value, fill=color,
                              font=ImageFont.truetype("DejaVuSans.ttf", size))


def stamp(canvas, image, center, bounds):
    fitted = ImageOps.contain(image, bounds, Image.Resampling.LANCZOS)
    canvas.alpha_composite(fitted, (round(center[0] - fitted.width / 2),
                                   round(center[1] - fitted.height / 2)))


def preview(geometry, output):
    manifest = json.loads((PACK / "manifest.json").read_text())
    images = {name: Image.open(PACK / asset["path"]).convert("RGBA")
              for name, asset in manifest["assets"].items()}
    # Only outline/center are used. C3617 is not assigned any of these materials.
    cell = next(item for item in json.loads(geometry.read_text())["hexes"] if item["id"] == "C3617")
    local = [(x - cell["center"][0], y - cell["center"][1]) for x, y in cell["corners"]]
    width = max(x for x, _ in local) - min(x for x, _ in local)
    height = max(y for _, y in local) - min(y for _, y in local)
    board = Image.new("RGBA", (1500, 850), "#f4efe4")
    text(board, (35, 24), "NORTH AFRICA / ORIGINAL TERRAIN LIBRARY", 31)
    text(board, (35, 70), "Painter artwork  •  9 ground materials + 31 independent transparent overlays", 21)
    text(board, (35, 106), "Synthetic art samples only. No geography, route, construction or movement assignments.", 18)
    for index, name in enumerate(SURFACES):
        x = 25 + index * 164
        board.alpha_composite(images[name].resize((145, 145), Image.Resampling.LANCZOS), (x, 170))
        words = name.replace("_", " ").title().split()
        for line, word in enumerate(words):
            text(board, (x, 324 + line * 21), word, 17)
    text(board, (35, 410), "COMPOSED SAMPLES / INDEPENDENT LAYERS", 27)
    cases = [
        ("Clear + road", "clear", [("road", (0, 0), (230, 25))], "Finished road"),
        ("Desert + railroad", "desert", [("railroad", (0, 0), (230, 25))], "Finished railroad"),
        ("Delta + major river", "delta", [("major_river", (0, 0), (230, 55))], "River is not baked in"),
        ("Desert + oasis", "desert", [("oasis", (0, 0), (115, 125))], "Independent point stamp"),
        ("Unfinished transport", "rough", [("road_unfinished", (0, -35), (230, 28)),
                                              ("railroad_unfinished", (0, 35), (230, 28))],
         "Explicit construction only"),
        ("Unknown geography", None, [("unknown", (0, 0), (150, 150))], "Not Clear / not Railroad"),
    ]
    for index, (title, surface, overlays, subtitle) in enumerate(cases):
        tile = Image.new("RGBA", (240, 280), manifest["unknown"]["fill"])
        if surface:
            tile = ImageOps.fit(images[surface], tile.size, method=Image.Resampling.LANCZOS)
        for name, (dx, dy), size in overlays:
            stamp(tile, images[name], (120 + dx, 140 + dy), size)
        polygon = [(120 + x * 2.4, 140 + y * 2.4) for x, y in local]
        mask = Image.new("L", tile.size)
        ImageDraw.Draw(mask).polygon(polygon, fill=255)
        tile.putalpha(ImageChops.multiply(tile.getchannel("A"), mask))
        ImageDraw.Draw(tile).line(polygon + [polygon[0]], fill="#69716e", width=2)
        board.alpha_composite(tile, (5 + 250 * index, 460))
        text(board, (12 + 250 * index, 741), title, 17)
        text(board, (12 + 250 * index, 768), subtitle, 14)
    text(board, (35, 813), f"Outline: exported geometry::corners(C3617), {width:.2f} × {height:.2f} board px. "
         "Shape only; samples enlarged 2.4×. No 512px-per-hex assumption.", 15)
    output.mkdir(parents=True, exist_ok=True)
    board.convert("RGB").save(output / "terrain-composed.png")

    overlays = [(name, image) for name, image in images.items() if name not in SURFACES]
    sheet = Image.new("RGBA", (1600, 1740), "#e7e9e9")
    text(sheet, (25, 20), "TRANSPARENT OVERLAYS / LIGHT + DARK BACKGROUNDS", 28)
    text(sheet, (25, 64), "All 31 extracted assets. Each pair uses the same RGBA PNG; backgrounds are inspection-only.", 20)
    for index, (name, image) in enumerate(overlays):
        x, y = (index % 4) * 400, 110 + (index // 4) * 200
        for side, color in enumerate(("#f3eee2", "#18232c")):
            panel = Image.new("RGBA", (192, 152), color)
            stamp(panel, image, (96, 76), (180, 140))
            sheet.alpha_composite(panel, (x + 4 + side * 196, y))
        text(sheet, (x + 8, y + 161), name.replace("_", " "), 17)
    sheet.convert("RGB").save(output / "terrain-alpha-check.png")
    print(f"Rendered {output / 'terrain-composed.png'} and {output / 'terrain-alpha-check.png'}")
    print(f"Geometry source: C3617 outline only, {width:.5f} × {height:.5f} board px; {len(overlays)} RGBA pairs")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, epilog=(
        "Example: uv run scripts/terrain-art/preview.py --geometry /tmp/map-model.json "
        "--output .amp/in/artifacts"))
    parser.add_argument("--geometry", type=Path, required=True, help="JSON from cna-map map-model")
    parser.add_argument("--output", type=Path, required=True, help="Directory for two review PNGs")
    args = parser.parse_args()
    preview(args.geometry, args.output)
