"""Require real test discovery and fail closed on incomplete validation reports."""
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

mode, filename = sys.argv[1:3]
path = Path(filename)
if not path.is_file() or not path.stat().st_size:
    raise SystemExit(f"Missing report: {path}")
if mode == "junit":
    cases = list(ET.parse(path).iter("testcase"))
    if len(cases) < 14 or any(any(c.tag in {"failure", "error", "skipped"} for c in case) for case in cases):
        raise SystemExit("Expected at least 14 successful tests with no skips")
elif mode == "mutations":
    data = json.loads(path.read_text())
    counts = ["total_mutants", "missed", "timeout", "caught", "unviable", "success"]
    if any(type(data.get(key)) is not int or data[key] < 0 for key in counts):
        raise SystemExit("Missing or invalid mutation counts")
    if data["missed"] or data["timeout"] or data["success"]:
        raise SystemExit("Surviving or timed-out mutations require investigation")
    outcomes = data.get("outcomes")
    if not isinstance(outcomes, list) or not data.get("end_time") or data.get("cargo_mutants_version") != "27.1.0":
        raise SystemExit("Incomplete mutation report")
    if len(outcomes) != data["total_mutants"] + 1 or not any(o["scenario"] == "Baseline" and o["summary"] == "Success" for o in outcomes):
        raise SystemExit("Mutation baseline or discovery evidence is incomplete")
    if sum(data[key] for key in counts[1:]) != data["total_mutants"]:
        raise SystemExit("Mutation counters do not account for every candidate")
else:
    raise SystemExit("unknown report type")
