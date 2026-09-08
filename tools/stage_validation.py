"""Stage complete validation controls from a trusted Git revision before builds."""
import argparse
import io
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import tarfile
import tempfile

CONTROLS = (".cargo", ".config", "tools", "deny.toml")
HARNESS = ("package.json", "package-lock.json", "playwright.config.ts", "tests/browser", "tests/baselines")


def export_revision(root, revision, paths, destination):
    """Export regular Git files only; never follow archived links or traversal paths."""
    archive = subprocess.check_output(["git", "archive", revision, *paths], cwd=root)
    with tarfile.open(fileobj=io.BytesIO(archive)) as bundle:
        for entry in bundle:
            relative = PurePosixPath(entry.name)
            if relative.is_absolute() or ".." in relative.parts or "\\" in entry.name:
                raise ValueError("Unsafe trusted archive path")
            target = destination.joinpath(*relative.parts)
            if entry.isdir():
                target.mkdir(parents=True, exist_ok=True)
            elif entry.isfile():
                target.parent.mkdir(parents=True, exist_ok=True)
                with bundle.extractfile(entry) as source, target.open("wb") as output:
                    shutil.copyfileobj(source, output)
            else:
                raise ValueError("Validation controls must contain regular files only")


def replace_controls(root, trusted):
    """Replace whole owned trees so candidate-only aliases and Cargo targets vanish."""
    root = root.resolve(strict=True)
    for name in CONTROLS:
        source, target = trusted / name, root / name
        if not source.exists():
            raise ValueError(f"Trusted controls are incomplete: {name}")
        # Only these fixed immediate children may be removed. Unlink a candidate
        # link itself, never recurse through its destination outside the checkout.
        if target.is_symlink():
            target.unlink()
        elif getattr(target, "is_junction", lambda: False)():
            target.rmdir()
        elif target.exists():
            if target.resolve().parent != root:
                raise ValueError("Validation target escapes the checkout")
            if target.is_dir():
                shutil.rmtree(target)
            else:
                target.unlink()
        if source.is_dir():
            shutil.copytree(source, target)
        else:
            shutil.copyfile(source, target)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("revision")
    parser.add_argument("--harness", type=Path)
    args = parser.parse_args()
    if not re.fullmatch(r"[a-f0-9]{40}", args.revision):
        raise ValueError("Trusted revision must be a full commit hash")
    root = Path.cwd().resolve()
    git_root = Path(subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()).resolve()
    if root != git_root:
        raise ValueError("Stage validation from the checkout root")
    with tempfile.TemporaryDirectory(prefix="claimlands-controls-") as temporary:
        trusted = Path(temporary)
        export_revision(root, args.revision, CONTROLS, trusted)
        replace_controls(root, trusted)
    if args.harness:
        # A fresh directory excludes candidate node_modules, .npmrc, lockfiles,
        # test configuration and module-resolution parents inside the checkout.
        args.harness.mkdir(parents=True, exist_ok=False)
        export_revision(root, args.revision, HARNESS, args.harness)


if __name__ == "__main__":
    main()
