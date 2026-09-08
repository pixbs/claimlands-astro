"""Regression checks for native smoke false positives; no device is required."""

import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "lifecycle", Path(__file__).parents[1] / "verify-lifecycle.py"
)
assert spec is not None and spec.loader is not None
lifecycle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lifecycle)


class LifecycleEvidenceTests(unittest.TestCase):
    def test_rendered_lifecycle(self):
        lifecycle.verify(
            "ClaimLands resumed=1\nClaimLands ready=true\n"
            "ClaimLands suspended=2\nClaimLands resumed=3\nClaimLands ready=true\n"
        )

    def test_missing_failed_and_stale_frames_are_rejected(self):
        traces = {
            "empty evidence": "",
            "one frame": "ClaimLands resumed=1\nClaimLands ready=true\n",
            "ready without a resume": "ClaimLands ready=true\nClaimLands ready=true\n",
            "duplicate readiness": (
                "ClaimLands resumed=1\nClaimLands ready=true\nClaimLands ready=true\n"
                "ClaimLands suspended=2\nClaimLands resumed=3\n"
            ),
            "missed frame before next generation": (
                "ClaimLands resumed=1\nClaimLands resumed=2\nClaimLands ready=true\n"
            ),
            "suspend before frame": "ClaimLands resumed=1\nClaimLands suspended=2\n",
            "native error after apparently good frames": (
                "ClaimLands resumed=1\nClaimLands ready=true\nClaimLands suspended=2\n"
                "ClaimLands resumed=3\nClaimLands ready=true\nClaimLands error=surface lost\n"
            ),
        }
        for reason, trace in traces.items():
            with self.subTest(reason=reason), self.assertRaises(ValueError):
                lifecycle.verify(trace)


if __name__ == "__main__":
    unittest.main()
