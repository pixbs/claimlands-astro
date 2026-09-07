"""Run the established mutation tool, with explicit zero-applicability evidence."""
import json
import subprocess
import sys
from pathlib import Path

args = ["cargo", "mutants", "--no-config", "-p", "claimlands-world", "-p", "claimlands-visuals"]
if len(sys.argv) > 1:
    args += ["--in-diff", sys.argv[1]]
listed = json.loads(subprocess.check_output(args + ["--list", "--json"], text=True))
if not isinstance(listed, list):
    raise SystemExit("Unexpected mutation discovery result")
Path("mutants.out").mkdir(exist_ok=True)
Path("mutants.out/applicability.json").write_text(json.dumps({"discovered": len(listed)}))
if listed:
    subprocess.run(args + ["--timeout", "120", "--jobs", "2"], check=True)
    subprocess.run([sys.executable, str(Path(__file__).with_name("verify_reports.py")), "mutations", "mutants.out/outcomes.json"], check=True)
else:
    print("No mutations apply to changed core lines; discovery completed successfully.")
