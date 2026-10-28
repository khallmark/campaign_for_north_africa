"""Synthetic source-image controls; run with requirements-map.txt installed."""
import importlib.util
import json
from pathlib import Path
import unittest

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("extract_map", ROOT / "scripts/extract-map.py")
extract_map = importlib.util.module_from_spec(spec)
spec.loader.exec_module(extract_map)


class SourceRouteTests(unittest.TestCase):
    def setUp(self):
        self.rules = json.loads((ROOT / "data/map/extraction-rules.json").read_text())
        self.side = {"hex": "C2410", "other": "C2411", "direction": "east",
                     "edge": [[120, 85], [120, 155]]}
        self.image = Image.new("RGB", (250, 250), tuple(self.rules["inks"]["background"]))

    def findings(self):
        return extract_map.examine_side(self.image, self.side, self.rules)["findings"]

    def test_antialiased_track_does_not_require_exact_palette_pixels(self):
        foreground = extract_map.translucent_foreground(self.rules, "rail_track")
        background = self.rules["inks"]["background"]
        color = tuple(round(float(.6 * f + .4 * b)) for f, b in zip(foreground, background))
        draw = ImageDraw.Draw(self.image)
        for x in range(20, 220, 20):
            draw.line((x, 120, x + 13, 120), fill=color, width=2)
        found = next(f for f in self.findings() if f["ink"] == "rail_track")
        self.assertEqual(found["pixels"], 0)
        self.assertEqual(found["value"], "track")

    def test_corner_bend_is_not_promoted_from_its_interior_chord(self):
        draw = ImageDraw.Draw(self.image)
        # The strokes reach x=120 only in the bottom corner exclusion zone.
        # A chord between distant dashes falsely crosses the interior near y=137.
        for start, end in (
            ((113, 73), (114, 86)), ((114, 92), (115, 105)),
            ((115, 111), (116, 124)), ((117, 130), (118, 143)),
            ((119, 149), (122, 162)), ((125, 168), (130, 181)),
            ((133, 187), (138, 200)),
        ):
            draw.line((*start, *end), fill=tuple(self.rules["inks"]["rail_track"]), width=2)
        self.assertFalse(any(f["value"] == "track" for f in self.findings()))

    def test_repeated_hollow_capsules_are_unfinished_not_completed_road(self):
        draw = ImageDraw.Draw(self.image)
        for x in range(20, 220, 20):
            draw.rectangle((x, 118, x + 13, 122), outline=tuple(self.rules["inks"]["road"]), width=1)
        found = next(f for f in self.findings() if f["ink"] == "road")
        self.assertEqual(found["depiction"], "unfinished_road")
        self.assertIsNone(found["value"])

    def test_road_capsule_clipped_at_terminus_still_requires_crossing_ink(self):
        draw = ImageDraw.Draw(self.image)
        color = tuple(self.rules["inks"]["road"])
        for x in (60, 80, 100, 116):
            draw.rectangle((x, 118, x + 13, 122), outline=color, width=1)
        draw.line((129, 117, 129, 123), fill=tuple(self.rules["inks"]["background"]), width=1)
        self.assertTrue(any(f.get("depiction") == "unfinished_road" for f in self.findings()))

    def test_solid_dashes_single_capsule_and_continuous_road_are_not_unfinished(self):
        for shape in ("solid_dashes", "single_capsule", "continuous_road", "distant_capsules", "ends_before_side"):
            with self.subTest(shape=shape):
                self.setUp()
                draw = ImageDraw.Draw(self.image)
                color = tuple(self.rules["inks"]["road"])
                if shape == "solid_dashes":
                    for x in range(20, 220, 20):
                        draw.rectangle((x, 118, x + 13, 122), fill=color)
                elif shape == "single_capsule":
                    draw.rectangle((113, 118, 127, 122), outline=color, width=1)
                elif shape in ("distant_capsules", "ends_before_side"):
                    positions = (20, 80, 140, 200) if shape == "distant_capsules" else (40, 60, 80, 100)
                    for x in positions:
                        draw.rectangle((x, 118, x + 13, 122), outline=color, width=1)
                else:
                    draw.line((20, 118, 220, 118), fill=color, width=1)
                    draw.line((20, 122, 220, 122), fill=color, width=1)
                findings = self.findings()
                self.assertFalse(any(f.get("depiction") == "unfinished_road" for f in findings))
                if shape == "continuous_road":
                    self.assertTrue(any(f["value"] == "road" for f in findings))


if __name__ == "__main__":
    unittest.main()
