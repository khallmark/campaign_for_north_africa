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
        rules = {"inks": {"water": [138, 181, 207], "grid": [51, 53, 51]}}
        crop = np.full((24, 36, 3), rules["inks"]["water"], dtype=np.uint8)
        # Dark center plus two independently calculated water/grid blend edges.
        crop[:, 8] = [116, 149, 168]
        crop[:, 9] = rules["inks"]["grid"]
        crop[:, 10] = [129, 168, 191]
        crop[:, 22:25] = [251, 250, 239]
        mask = water.water_mask(crop, rules)
        self.assertTrue(np.all(mask[:, :22] == 255))
        self.assertTrue(np.all(mask[:, 22:25] == 0))
        self.assertTrue(np.all(mask[:, 25:] == 255))


if __name__ == "__main__":
    unittest.main()
