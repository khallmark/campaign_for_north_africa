# /// script
# requires-python = ">=3.11"
# dependencies = ["Pillow==13.0.0", "numpy==2.4.6", "opencv-python-headless==5.0.0.93", "scikit-image==0.26.0", "scipy==1.17.1"]
# ///
"""Display geometry controls: uv run scripts/test_source_paths.py (no private scan)."""
import importlib.util
from pathlib import Path
import unittest

import cv2
import numpy as np

spec = importlib.util.spec_from_file_location("trace_paths", Path(__file__).with_name("trace-map-paths.py"))
trace = importlib.util.module_from_spec(spec)
spec.loader.exec_module(trace)


class SourcePathsTests(unittest.TestCase):
    def test_paths_stop_at_real_gaps_and_branch_instead_of_auto_joining(self):
        mask = np.zeros((240, 360), dtype=np.uint8)
        cv2.line(mask, (10, 100), (160, 100), 255, 3)
        cv2.line(mask, (180, 100), (340, 100), 255, 3)
        cv2.line(mask, (250, 100), (250, 210), 255, 3)
        paths = trace.chains(mask)
        self.assertEqual(len(paths), 4)
        for path in paths:
            self.assertFalse(path[:, 0].min() < 160 and path[:, 0].max() > 180)
            # Each branch ends at the junction, never selects an arbitrary turn.
            if path[:, 1].max() > 150:
                self.assertLess(np.ptp(path[:, 0]), 4)

    def test_enclosed_raster_pinholes_do_not_fragment_a_road_or_bridge_a_gap(self):
        mask = np.zeros((40, 400), dtype=np.uint8)
        mask[16:25, 10:390] = 255
        for x in range(20, 380, 12):
            mask[19:21, x:x+2] = 0
        # A one-pixel gap connects to the background, unlike enclosed pinholes.
        # Morphological closing would erase this source construction gap.
        mask[:, 193] = 0
        paths = trace.chains(mask)
        self.assertEqual(len(paths), 2)
        spans = sorted((path[:, 0].min(), path[:, 0].max()) for path in paths)
        self.assertLess(spans[0][0], 20)
        self.assertGreater(spans[0][1], 180)
        self.assertLess(spans[0][1], 193)
        self.assertGreater(spans[1][0], 193)
        self.assertLess(spans[1][0], 205)
        self.assertGreater(spans[1][1], 380)

    def test_rail_traces_require_sleepers_not_a_crossing_grid_line(self):
        mask = np.zeros((220, 360), dtype=np.uint8)
        cv2.line(mask, (10, 100), (340, 100), 255, 3)
        for x in range(30, 330, 20):
            cv2.line(mask, (x, 94), (x, 106), 255, 2)
        cv2.line(mask, (180, 10), (180, 210), 255, 1)
        paths = trace.chains(mask, railroad=True)
        self.assertEqual(len(paths), 1)
        self.assertGreater(sum(np.ptp(path[:, 0]) for path in paths), 250)
        for path in paths:
            self.assertLess(np.ptp(path[:, 1]), 5)

    def test_diagonal_sleepers_preserve_the_spine_and_real_branch(self):
        mask = np.zeros((330, 450), dtype=np.uint8)
        cv2.line(mask, (20, 280), (420, 80), 255, 3)
        cv2.line(mask, (220, 180), (220, 25), 255, 3)
        for x in range(40, 410, 16):
            y = 290 - x // 2
            cv2.line(mask, (x-3, y-6), (x+3, y+6), 255, 2)
        for y in range(40, 170, 16):
            cv2.line(mask, (214, y), (226, y), 255, 2)
        # Grid on both sides of the rail must not become an extra branch.
        cv2.line(mask, (315, 0), (315, 329), 255, 1)
        paths = trace.chains(mask, railroad=True)
        points = np.concatenate(paths)
        diagonal = abs(points[:, 1] - (290-points[:, 0]/2)) < 4
        branch = (abs(points[:, 0]-220) < 4) & (points[:, 1] <= 183)
        self.assertTrue(np.all(diagonal | branch))
        self.assertGreater(np.count_nonzero(diagonal), 360)
        # Sleeper pruning may remove the terminal tail (under 18 pixels).
        self.assertLessEqual(points[branch, 1].min(), 25 + 18)
        self.assertGreater(np.count_nonzero(branch), 120)

    def test_asymmetric_curve_preserves_endpoints_and_sampled_fit(self):
        raw = np.array([[x, 20 + .003 * x*x] for x in range(180)])
        segments, error = trace.fit(raw)
        np.testing.assert_allclose(segments[0][0], raw[0])
        np.testing.assert_allclose(segments[-1][-1], raw[-1], atol=.00001)
        self.assertLess(error, 1.5)
        self.assertGreater(len(segments), 3)
        for previous, current in zip(segments, segments[1:]):
            np.testing.assert_array_equal(previous[-1], current[0])

        # A supported right-angle branch must survive the fit, rather than
        # dropping its entire source chain because one broad quadratic cuts it.
        elbow = np.array([[x, 0] for x in range(100)] + [[99, y] for y in range(1, 121)])
        segments, error = trace.fit(elbow)
        self.assertLessEqual(error, 1.5)
        np.testing.assert_array_equal(segments[0][0], [0, 0])
        np.testing.assert_array_equal(segments[-1][-1], [99, 120])
        for a, b, c in np.asarray(segments):
            for t in np.linspace(0, 1, 50):
                x, y = (1-t)**2*a + 2*(1-t)*t*b + t*t*c
                self.assertLessEqual(min(abs(y), abs(x-99)), 1.5)


if __name__ == "__main__":
    unittest.main()
