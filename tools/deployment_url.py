"""Extract a project-scoped immutable Pages URL from successful Wrangler output."""
import re
import sys
from pathlib import Path

urls = re.findall(r"https://[a-f0-9]{8,}\.claimlands-astro\.pages\.dev", Path(sys.argv[1]).read_text())
if not urls:
    raise SystemExit("Wrangler did not return an immutable game deployment URL")
with Path(sys.argv[2]).open("a") as output:
    output.write(f"url={urls[-1]}\n")
print(f"Game preview: {urls[-1]}")
