"""Require a rendered frame in every activated native renderer generation."""

import re
import sys
from pathlib import Path


def verify(log: str) -> None:
    """Reject missing, stale, failed, or interrupted renderer evidence."""
    if re.search(r"ClaimLands error=|panicked at|fatal runtime error", log):
        raise ValueError("native game reported an error")
    awaiting_frame = False
    rendered_generations = 0
    suspensions = 0
    for event, value in re.findall(r"ClaimLands (resumed|suspended|ready)=([^\s]+)", log):
        if event == "resumed":
            if awaiting_frame:
                raise ValueError("renderer resumed again without rendering its previous generation")
            awaiting_frame = True
        elif event == "ready" and value == "true" and awaiting_frame:
            awaiting_frame = False
            rendered_generations += 1
        elif event == "suspended":
            if awaiting_frame:
                raise ValueError("renderer suspended before its first successful frame")
            suspensions += 1
    if awaiting_frame:
        raise ValueError("last renderer generation has no successful frame")
    if rendered_generations < 2 or suspensions < 1:
        raise ValueError("missing rendered launch and resume with an intervening suspension")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: python3 platform/verify-lifecycle.py NATIVE_LOG")
    try:
        verify(Path(sys.argv[1]).read_text(encoding="utf-8", errors="replace"))
    except (OSError, ValueError) as error:
        raise SystemExit(str(error)) from error
    print("Validated rendered native lifecycle generations")
