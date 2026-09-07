"""Publish head-specific gates from the trusted workflow only."""
import json
import os
import subprocess
import sys
import policy


def status(sha, context, state, description, url=None):
    data = {"state": state, "context": context, "description": description[:140]}
    if url:
        data["target_url"] = url
    subprocess.run(["gh", "api", "--method", "POST", f"repos/pixbs/claimlands-astro/statuses/{sha}", "--input", "-"], input=json.dumps(data), text=True, check=True, stdout=subprocess.DEVNULL)


def main():
    sha = os.environ["HEAD_SHA"]
    pr = os.environ.get("PR_NUMBER", "")
    run_url = f"https://github.com/pixbs/claimlands-astro/actions/runs/{os.environ['GITHUB_RUN_ID']}"
    mode = sys.argv[1]
    if mode == "start":
        for context in ["quality", "publication-policy", "game-preview"]:
            status(sha, context, "pending", "Validation running for this revision", run_url)
        try:
            if pr:
                if policy.check_pr(int(pr)) != sha:
                    raise ValueError("The PR head changed; refusing a stale validation.")
            status(sha, "publication-policy", "success", "Complete introduced history and current metadata passed", run_url)
        except Exception:
            status(sha, "publication-policy", "failure", "Publication metadata rejected or could not be verified", run_url)
            raise
    elif mode == "finish":
        results = json.loads(os.environ["JOB_RESULTS"])
        required = ["native", "browser", "android", "ios", "mutations", "visual"]
        good = all(results.get(name, {}).get("result") == "success" for name in required)
        metadata = True
        if pr:
            try:
                metadata = policy.check_pr(int(pr)) == sha
            except Exception:
                metadata = False
        status(sha, "publication-policy", "success" if metadata else "failure", "Current metadata revalidated" if metadata else "Metadata changed or failed policy", run_url)
        status(sha, "quality", "success" if good and metadata else "failure", "All mandatory jobs passed" if good and metadata else "Required validation failed, skipped, or unavailable", run_url)
        preview_ok = results.get("preview", {}).get("result") == "success" and metadata
        status(sha, "game-preview", "success" if preview_ok else "failure", "Deployed revision rendered and responded" if preview_ok else "Preview failed, stale, skipped, or unavailable", os.environ.get("PREVIEW_URL") or run_url)
        if not good or not preview_ok or not metadata:
            raise SystemExit(1)
        from pathlib import Path
        Path("validation-manifest.json").write_text(json.dumps({
            "schema_version": 1, "repository": "pixbs/claimlands-astro",
            "pr_number": int(pr) if pr else 0,
            "head_sha": sha, "base_sha": os.environ["BASE_SHA"],
            "trusted_sha": os.environ["TRUSTED_SHA"],
            "run_id": int(os.environ["GITHUB_RUN_ID"]),
            "run_attempt": int(os.environ["GITHUB_RUN_ATTEMPT"]),
            "workflow_path": ".github/workflows/validate.yml",
        }))
    else:
        raise ValueError("unknown status operation")


if __name__ == "__main__":
    main()
