"""Validate publication metadata, never source text. Used by hooks and trusted CI."""
import argparse
import json
import re
import subprocess
import sys
import unicodedata
from pathlib import Path

FORBIDDEN = re.compile(r"claude|codex|kimi|chatgpt|gemini|copilot|deepseek|gpt[-_ ]?[0-9]|anthropic", re.IGNORECASE)
BRANCH = re.compile(r"(?:feat|fix|refactor|test|docs|chore)/[1-9][0-9]*-[a-z0-9]+(?:-[a-z0-9]+)*\Z")
SUBJECT = re.compile(r"(?:feat|fix|refactor|test|docs|chore|build|ci|perf|revert)(?:\([a-z0-9-]+\))?!?: .+")


def validate_text(text):
    if FORBIDDEN.search(unicodedata.normalize("NFKC", text)):
        raise ValueError("Publication metadata contains a prohibited name. Correct it before publishing.")


def validate_message(message):
    validate_text(message)
    subject = message.splitlines()[0] if message.splitlines() else ""
    if not SUBJECT.fullmatch(subject) or len(subject) > 72:
        raise ValueError("Use a Conventional Commit subject of at most 72 characters.")
    if re.search(r"(?im)^co-authored-by:.*(?:\[bot\]|noreply|agent|assistant)", message):
        raise ValueError("Automated co-author trailers are forbidden.")


def validate_branch(branch):
    validate_text(branch)
    if not BRANCH.fullmatch(branch):
        raise ValueError("Branch must use type/issue-description naming.")


def check_push(lines):
    """Inspect every pushed ref, including tags and non-current branches."""
    for line in lines:
        local_ref, local_sha, remote_ref, remote_sha = line.split()
        validate_text(local_ref + "\n" + remote_ref)
        if local_sha == "0" * 40:
            continue
        if remote_ref.startswith("refs/heads/") and remote_ref != "refs/heads/main":
            validate_branch(remote_ref.removeprefix("refs/heads/"))
        if remote_ref.startswith("refs/tags/"):
            if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+(?:-[a-z0-9.-]+)?", remote_ref.removeprefix("refs/tags/")):
                raise ValueError("Tags must be neutral semantic versions, e.g. v0.1.0.")
            validate_text(run("git", "for-each-ref", "--format=%(contents)%0a%(taggername)%0a%(taggeremail)", local_ref))
        base = remote_sha if remote_sha != "0" * 40 else "origin/main"
        for sha in run("git", "rev-list", f"{base}..{local_sha}").splitlines():
            validate_message(run("git", "show", "-s", "--format=%B", sha))
            validate_text(run("git", "show", "-s", "--format=%an%n%ae%n%cn%n%ce", sha))


def run(*args):
    return subprocess.check_output(args, text=True, encoding="utf-8")


def api(path):
    return json.loads(run("gh", "api", path))


def check_pr(number, repo="pixbs/claimlands-astro"):
    pr = api(f"repos/{repo}/pulls/{int(number)}")
    validate_message(pr["title"])
    validate_text(pr["body"] or "")
    validate_text(pr["head"]["ref"] + "\n" + pr["base"]["ref"])
    if not BRANCH.fullmatch(pr["head"]["ref"]):
        raise ValueError("Branch must use type/issue-description naming.")
    sha = pr["head"]["sha"]
    main = api(f"repos/{repo}/git/ref/heads/main")["object"]["sha"]
    commits = []
    page = 1
    while True:
        comparison = api(f"repos/{repo}/compare/{main}...{sha}?per_page=100&page={page}")
        commits.extend(comparison["commits"])
        if len(commits) >= comparison["total_commits"]:
            break
        if not comparison["commits"]:
            raise ValueError("Incomplete commit history; refusing to pass.")
        page += 1
    if not commits:
        raise ValueError("PR introduces no commits.")
    for item in commits:
        commit = item["commit"]
        validate_message(commit["message"])
        for role in ("author", "committer"):
            validate_text(commit[role]["name"] + "\n" + commit[role]["email"])
    fresh = api(f"repos/{repo}/pulls/{int(number)}")
    if (fresh["head"]["sha"], fresh["base"]["sha"], fresh["base"]["ref"], fresh["title"], fresh["body"]) != (sha, pr["base"]["sha"], pr["base"]["ref"], pr["title"], pr["body"]):
        raise ValueError("PR changed during validation; rerun the check.")
    return sha


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--text")
    parser.add_argument("--message", type=Path)
    parser.add_argument("--range")
    parser.add_argument("--pr", type=int)
    parser.add_argument("--branch")
    parser.add_argument("--branch-only")
    parser.add_argument("--pre-push", action="store_true")
    args = parser.parse_args()
    if args.pre_push:
        check_push(sys.stdin)
    elif args.branch_only is not None:
        validate_branch(args.branch_only)
    elif args.text is not None:
        validate_text(args.text)
    elif args.message:
        validate_message(args.message.read_text(encoding="utf-8"))
    elif args.pr:
        print(check_pr(args.pr))
    else:
        branch = args.branch or run("git", "branch", "--show-current").strip()
        validate_text(branch)
        if branch != "main" and not BRANCH.fullmatch(branch):
            raise ValueError("Branch must use type/issue-description naming.")
        revision_range = vars(args)["range"] or "origin/main..HEAD"
        for sha in run("git", "rev-list", revision_range).splitlines():
            validate_message(run("git", "show", "-s", "--format=%B", sha))
            validate_text(run("git", "show", "-s", "--format=%an%n%ae%n%cn%n%ce", sha))
    print("Publication policy passed")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
