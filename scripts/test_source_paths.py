"""Display geometry invariants; no private source images needed."""
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

    def test_asymmetric_curve_preserves_endpoints_and_sampled_fit(self):
        raw = np.array([[x, 20 + .003 * x*x] for x in range(180)])
        segments, error = trace.fit(raw)
        np.testing.assert_allclose(segments[0][0], raw[0])
        np.testing.assert_allclose(segments[-1][-1], raw[-1], atol=.00001)
        self.assertLess(error, 1.5)
        self.assertGreater(len(segments), 3)
        for previous, current in zip(segments, segments[1:]):
            np.testing.assert_array_equal(previous[-1], current[0])


if __name__ == "__main__":
    unittest.main()
