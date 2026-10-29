"""Water display tracing must not turn pale land gaps into water."""
import importlib.util
from pathlib import Path
import unittest

import numpy as np

spec = importlib.util.spec_from_file_location("water", Path(__file__).with_name("trace-map-water.py"))
water = importlib.util.module_from_spec(spec)
spec.loader.exec_module(water)


class WaterTests(unittest.TestCase):
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
