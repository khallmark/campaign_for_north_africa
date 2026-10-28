"""Pixel contracts for blended original terrain, independent of GPU rendering."""
import unittest

import numpy as np
from PIL import Image

from raster import cells_from_model, render_tile


class GroundTests(unittest.TestCase):
    def test_blends_cross_tiles_but_keeps_unknown_and_unresolved_neutral(self):
        def cell(name, x0, x1):
            return {"id": name, "corners": [[x0, 0], [x1, 0], [x1, 80], [x0, 80]]}

        def record(hex_id, status, value):
            return {"kind": "terrain", "hex": hex_id,
                    "record": {"observation": {"status": status, "value": value}}}

        model = {"hexes": [cell("A", 0, 509), cell("B", 510, 899), cell("C", 900, 1023)],
                 "surveys": [{"records": [record("A", "reviewed", "desert"),
                                           record("B", "provisional", "rough"),
                                           record("C", "fixture", "rough"),
                                           record("C", "unresolved", "desert")]}]}
        cells = cells_from_model(model, {"desert": {"surface": "sand"}, "rough": {"surface": "rock"}})
        self.assertEqual([c["id"] for c in cells], ["A", "B"])
        textures = {"sand": np.full((13, 17, 3), (240, 180, 60), dtype=np.uint8),
                    "rock": np.full((19, 11, 3), (40, 80, 100), dtype=np.uint8),
                    "unmapped": np.full((17, 23, 3), (217, 215, 206), dtype=np.uint8)}
        whole = render_tile((0, 0, 1024, 80), cells, textures)
        stitched = Image.new("RGB", (1024, 80))
        for x in (0, 512):
            stitched.paste(render_tile((x, 0, x + 512, 80), cells, textures), (x, 0))
        np.testing.assert_array_equal(np.asarray(whole), np.asarray(stitched))
        self.assertEqual(whole.getpixel((100, 40)), (240, 180, 60))
        self.assertEqual(whole.getpixel((800, 40)), (40, 80, 100))
        self.assertEqual(whole.getpixel((901, 40)), (217, 215, 206))
        # The blend lies between the independently chosen material colors;
        # it is neither an inset gray gap nor an abrupt material cut.
        self.assertTrue(40 < whole.getpixel((510, 40))[0] < 240)
        self.assertLess(abs(whole.getpixel((511, 40))[0] - whole.getpixel((512, 40))[0]), 10)
        # A cell with two supported observations keeps both materials, rather
        # than letting the final survey silently replace the first one.
        model["surveys"][0]["records"].append(record("A", "provisional", "rough"))
        mixed = cells_from_model(model, {"desert": {"surface": "sand"}, "rough": {"surface": "rock"}})
        self.assertEqual(render_tile((0, 0, 1024, 80), mixed, textures).getpixel((100, 40)),
                         (140, 130, 80))
        # Nonuniform, asymmetric textures catch accidental per-tile restarts
        # that constant-color material tests cannot detect.
        pixels = np.arange(320 * 320 * 3, dtype=np.uint32).reshape(320, 320, 3)
        textures = {"sand": (pixels % 251).astype(np.uint8),
                    "rock": ((pixels // 7 + 51) % 237).astype(np.uint8),
                    "unmapped": ((pixels // 11 + 31) % 229).astype(np.uint8)}
        whole = render_tile((0, 0, 1024, 80), cells, textures)
        for x in (0, 512):
            stitched.paste(render_tile((x, 0, x + 512, 80), cells, textures), (x, 0))
        np.testing.assert_array_equal(np.asarray(whole), np.asarray(stitched))
        # Unknown pixels use the same neutral texture with or without nearby
        # classified terrain. They never inherit its blurred material.
        neutral = render_tile((0, 0, 1024, 80), [], textures)
        np.testing.assert_array_equal(np.asarray(whole)[:, 900:], np.asarray(neutral)[:, 900:])


if __name__ == "__main__":
    unittest.main()
