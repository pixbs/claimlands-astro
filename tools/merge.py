"""Merge an owner-approved revision using only the current main checkout's policy."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import sys
import types

REPOSITORY = "pixbs/claimlands-astro"
SHA = re.compile(r"[a-f0-9]{40}\Z")


def require(condition, message):
    if not condition:
        raise ValueError(message)


def read(root, *args):
    return subprocess.check_output(args, cwd=root, text=True, encoding="utf-8")


def run(root, *args):
    subprocess.run(args, cwd=root, check=True)


def github(path):
    return json.loads(subprocess.check_output(["gh", "api", "--method", "GET", path], text=True, encoding="utf-8"))


def verify_checkout(root, api=github, capture=read):
    """Reject a feature branch, stale main, or any tracked/untracked work before merging."""
    root = root.resolve()
    actual_root = Path(capture(root, "git", "rev-parse", "--show-toplevel").strip()).resolve()
    require(actual_root == root, "The merge entry point must be inside its own repository checkout")
    branch = capture(root, "git", "branch", "--show-current").strip()
    require(branch in {"", "main"}, "Candidate checkout rejected; use an isolated detached checkout of current main")
    current = capture(root, "git", "rev-parse", "HEAD").strip()
    main = api(f"repos/{REPOSITORY}/git/ref/heads/main")["object"]["sha"]
    require(SHA.fullmatch(main) and current == main, "Checkout is stale or is not the current GitHub main revision")
    dirty = capture(root, "git", "-c", "core.fsmonitor=false", "status", "--porcelain=v1", "--untracked-files=all", "--ignore-submodules=none")
    require(not dirty.strip(), "Merge checkout is dirty; create a fresh isolated checkout of current main")
    return main


def load_trusted_module(root, revision, name):
    # Load verified Git source directly: isolated Python does not add this directory
    # to sys.path, and ignored bytecode caches must not replace trusted source.
    source = read(root, "git", "show", f"{revision}:tools/{name}.py")
    module = types.ModuleType(name)
    module.__file__ = str(root / "tools" / f"{name}.py")
    exec(compile(source, module.__file__, "exec"), module.__dict__)
    return module


def merge_approved(pr, approved_sha, root, api=github, capture=read, execute=run, load=load_trusted_module):
    require(type(pr) is int and pr > 0, "PR number must be positive")
    require(SHA.fullmatch(approved_sha), "Approved SHA must be a full lowercase commit hash")
    trusted_sha = verify_checkout(root, api, capture)
    # Also catch a modified entry point hidden by a local assume-unchanged flag.
    source = capture(root, "git", "show", f"{trusted_sha}:tools/merge.py")
    require(source == (root / "tools" / "merge.py").read_text(encoding="utf-8"), "Merge entry point differs from trusted main")
    policy = load(root, trusted_sha, "policy")
    validation = load(root, trusted_sha, "verify_validation")
    require(policy.check_pr(pr) == approved_sha, "The approved PR head has changed")
    evidence = validation.verify(pr, approved_sha)
    execute(root, "gh", "pr", "checks", str(pr), "--repo", REPOSITORY, "--required")
    require(policy.check_pr(pr) == approved_sha, "The approved PR head changed during verification")
    require(verify_checkout(root, api, capture) == trusted_sha, "Trusted main changed during verification")
    print(f"Verified trusted validation: {evidence}")
    execute(root, "gh", "pr", "merge", str(pr), "--repo", REPOSITORY, "--rebase", "--match-head-commit", approved_sha)


def main():
    require(sys.flags.isolated, "Invoke the owner entry point with python -I tools/merge.py")
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pr", type=int, required=True)
    parser.add_argument("--approved-sha", required=True, help="Full head SHA explicitly accepted by the owner")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    require(Path.cwd().resolve() == root, "Run the entry point from its isolated main checkout directory")
    # The SHA argument records approval; this program cannot grant owner approval.
    merge_approved(args.pr, args.approved_sha, root)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Merge refused: {error}") from error
