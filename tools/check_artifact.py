"""Fail closed on unexpected deployment files, remote runtime imports, or oversize assets."""
from pathlib import Path
import sys


def validate(root):
    required = {"index.html", "claimlands.js", "claimlands_bg.wasm", "revision.txt", "_headers", "THIRD_PARTY.txt"}
    paths = list(root.rglob("*"))
    files = {p.relative_to(root).as_posix() for p in paths if p.is_file()}
    if not required <= files:
        raise ValueError(f"Missing game build files: {sorted(required - files)}")
    for path in paths:
        if path.is_symlink():
            raise ValueError("Deployment symlinks are forbidden")
        if not path.is_file():
            continue
        if path.stat().st_size >= 25 * 1024 * 1024:
            raise ValueError(f"Asset exceeds Pages size limit: {path.name}")
        if path.suffix not in {".html", ".js", ".wasm", ".txt", ".ts"} and path.name != "_headers":
            raise ValueError(f"Unexpected deployment file: {path.name}")
        if path.name in {"_worker.js", "_routes.json"}:
            raise ValueError("Preview must contain static game assets only")
        if path.suffix in {".html", ".js"}:
            text = path.read_text(encoding="utf-8").lower()
            if any(marker in text for marker in ["three.js", "three.min.js", "window.planet", "hex-planet.html", "cdnjs.cloudflare.com"]):
                raise ValueError("Prototype runtime entered the game artifact")
    if (root / "claimlands_bg.wasm").read_bytes()[:4] != b"\0asm":
        raise ValueError("Invalid WebAssembly module")
    if not (root / "revision.txt").read_text().strip():
        raise ValueError("Missing revision identity")


if __name__ == "__main__":
    validate(Path(sys.argv[1]))
    print("Game artifact passed")
