"""Water display tracing must not turn pale land gaps into water."""
import importlib.util
from pathlib import Path
import unittest

import numpy as np
from PIL import Image

spec = importlib.util.spec_from_file_location("water", Path(__file__).with_name("trace-map-water.py"))
water = importlib.util.module_from_spec(spec)
spec.loader.exec_module(water)


class WaterTests(unittest.TestCase):
    def test_scarp_bands_keep_their_printed_fringe_but_reject_thin_route_ink(self):
        mask = np.zeros((80, 220), dtype=np.uint8)
        mask[10:24, 10:160] = 255
        # Uneven source-facing edge and a real pale interruption stay intact.
        mask[24:29, 35:40] = 255
        mask[24:33, 70:75] = 255
        mask[8:35, 95:98] = 0
        # Detached printed speckles remain separate, not joined to the face.
        mask[30:33, 50:53] = 255
        mask[37:41, 80:84] = 255
        # A substantial thin cap can be separated by a printed grid line.
        mask[2:7, 165:185] = 255
        expected = mask.copy()
        # Escarpment-like antialiasing on a long route is not a broad scarp.
        mask[50:52, 10:170] = 255
        # Same-palette lettering can contain 5px cores, unlike rail fringes.
        mask[55:68, 20:45] = 255
        mask[60:63, 25:40] = 0
        # A compact antialias fragment near a scarp still belongs to its rail.
        mask[30:32, 166:170] = 255
        # A recovered speckle cannot seed recovery farther from the broad face.
        mask[54:57, 50:53] = 255
        # An isolated gray dot away from a band is not evidence of a scarp.
        mask[55:58, 180:183] = 255
        pixels = np.full((*mask.shape, 3), [251, 250, 239], dtype=np.uint8)
        pixels[mask > 0] = [94, 97, 98]
        pixels[29:33, 171:173] = [84, 88, 89]
        pixels[1:8, 186:188] = [51, 53, 51]
        result = water.escarpment_mask(mask, Image.fromarray(pixels))
        np.testing.assert_array_equal(result, expected)

    def test_repairs_grid_antialiasing_but_not_land_between_water(self):
        rules = {"inks": {"water": [138, 181, 207], "wadi": [127, 148, 142], "grid": [51, 53, 51]}}
        for ink, fringes in (("water", ([116, 149, 168], [129, 168, 191])),
                             ("wadi", ([108, 124, 119], [119, 138, 133]))):
            with self.subTest(ink=ink):
                crop = np.full((24, 36, 3), rules["inks"][ink], dtype=np.uint8)
                # Dark center plus independently calculated grid blend edges.
                crop[:, 8] = fringes[0]
                crop[:, 9] = rules["inks"]["grid"]
                crop[:, 10] = fringes[1]
                crop[:, 22:25] = [251, 250, 239]
                mask = water.channel_mask(crop, rules, ink)
                self.assertTrue(np.all(mask[:, :22] == 255))
                self.assertTrue(np.all(mask[:, 22:25] == 0))
                self.assertTrue(np.all(mask[:, 25:] == 255))
                other = "wadi" if ink == "water" else "water"
                self.assertFalse(np.any(water.channel_mask(crop, rules, other)))


if __name__ == "__main__":
    unittest.main()
