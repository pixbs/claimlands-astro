"""Verify trusted Actions execution before an owner-approved merge; read-only API calls."""
import argparse
import io
import json
import re
import subprocess
import zipfile

REPOSITORY = "pixbs/claimlands-astro"
WORKFLOW = ".github/workflows/validate.yml"
REQUIRED_JOBS = {"policy", "native", "browser", "visual", "mutations", "android", "ios", "deploy", "preview", "gate"}
SHA = re.compile(r"[a-f0-9]{40}\Z")
RUN_URL = re.compile(r"https://github\.com/pixbs/claimlands-astro/actions/runs/([1-9][0-9]*)\Z")


def github_bytes(path):
    return subprocess.check_output(["gh", "api", "--method", "GET", path])


def github_json(path):
    return json.loads(github_bytes(path))


def require(condition, message):
    if not condition:
        raise ValueError(message)


def positive_int(value):
    return type(value) is int and value > 0


def all_items(api, path, key):
    items = []
    for page in range(1, 101):
        data = api(f"{path}?per_page=100&page={page}")
        total = data.get("total_count")
        batch = data.get(key)
        require(type(total) is int and total >= 0 and isinstance(batch, list), f"Invalid {key} listing")
        items.extend(batch)
        if len(items) == total:
            return items
        require(batch and len(items) < total, f"Incomplete {key} listing")
    raise ValueError(f"Too many {key} pages; refusing incomplete verification")


def read_manifest(archive):
    require(len(archive) <= 1_048_576, "Validation artifact is unexpectedly large")
    with zipfile.ZipFile(io.BytesIO(archive)) as bundle:
        entries = bundle.infolist()
        require(len(entries) == 1 and entries[0].filename == "validation-manifest.json", "Unexpected validation artifact contents")
        require(0 < entries[0].file_size <= 16_384, "Validation manifest is empty or too large")

        def unique_keys(pairs):
            record = {}
            for key, value in pairs:
                require(key not in record, f"Duplicate manifest field: {key}")
                record[key] = value
            return record

        manifest = json.loads(bundle.read(entries[0]), object_pairs_hook=unique_keys)
    require(isinstance(manifest, dict), "Validation manifest must be an object")
    return manifest


def pr_identity(pr):
    return (pr["head"]["sha"], pr["base"]["sha"], pr["base"]["ref"], pr["title"], pr.get("body"), pr["state"], pr.get("draft"))


def verify(pr_number, approved_sha, api=github_json, download=github_bytes):
    require(positive_int(pr_number), "PR number must be positive")
    require(SHA.fullmatch(approved_sha), "Approved SHA must be a full lowercase commit hash")
    prefix = f"repos/{REPOSITORY}"
    pr = api(f"{prefix}/pulls/{pr_number}")
    require(pr["head"]["sha"] == approved_sha, "The approved PR head has changed")
    require(pr["state"] == "open" and pr.get("draft") is False, "PR must be open and ready for review")
    require(pr["base"]["ref"] == "main" and pr["base"]["repo"]["full_name"] == REPOSITORY, "Only this repository's main branch may be merged")
    main_sha = api(f"{prefix}/git/ref/heads/main")["object"]["sha"]
    require(pr["base"]["sha"] == main_sha and SHA.fullmatch(main_sha), "PR base must match current main")

    statuses = api(f"{prefix}/commits/{approved_sha}/status").get("statuses", [])
    quality = [status for status in statuses if status.get("context") == "quality"]
    require(quality, "No quality status identifies the validation run")
    latest = max(quality, key=lambda status: status["id"])
    require(latest.get("state") == "success", "Latest quality status is not successful")
    match = RUN_URL.fullmatch(latest.get("target_url") or "")
    require(match, "Quality status must link the actual repository Actions run")
    run_id = int(match.group(1))
    run = api(f"{prefix}/actions/runs/{run_id}")
    workflow = api(f"{prefix}/actions/workflows/validate.yml")
    require(workflow.get("path") == WORKFLOW and workflow.get("state") == "active", "The trusted validation workflow is missing or inactive")
    require(run.get("workflow_id") == workflow["id"] and run.get("path") == WORKFLOW, "Status refers to a different workflow")
    require(run.get("event") == "pull_request_target", "Validation must originate from the trusted default-branch PR workflow")
    require(run.get("repository", {}).get("full_name") == REPOSITORY, "Validation belongs to another repository")
    # Workflow-run head_sha is the candidate commit, unlike github.workflow_sha.
    require(run.get("head_sha") == approved_sha, "Validation tested a different candidate revision")
    require(run.get("status") == "completed" and run.get("conclusion") == "success", "Validation run has not completed successfully")
    attempt = run.get("run_attempt")
    require(positive_int(attempt), "Validation run has no valid attempt number")

    jobs = all_items(api, f"{prefix}/actions/runs/{run_id}/attempts/{attempt}/jobs", "jobs")
    names = [job.get("name") for job in jobs]
    require(REQUIRED_JOBS <= set(names), "Required jobs are missing from this attempt; rerun all jobs")
    require(len(names) == len(set(names)), "Ambiguous duplicate validation job names")
    require(all(job.get("run_id") == run_id and job.get("status") == "completed" and job.get("conclusion") == "success" for job in jobs), "A validation job failed, skipped or did not finish")

    artifacts = all_items(api, f"{prefix}/actions/runs/{run_id}/artifacts", "artifacts")
    matching = [artifact for artifact in artifacts if artifact.get("name") == f"validation-manifest-{approved_sha}"]
    require(len(matching) == 1, "Missing or ambiguous validation manifest artifact")
    artifact = matching[0]
    require(artifact.get("expired") is False and positive_int(artifact.get("id")), "Validation manifest artifact expired or has an invalid identity")
    require(artifact.get("workflow_run", {}).get("id") == run_id and artifact["workflow_run"].get("head_sha") == approved_sha, "Artifact provenance differs from the validation run")
    manifest = read_manifest(download(f"{prefix}/actions/artifacts/{artifact['id']}/zip"))
    expected = {
        "schema_version": 1,
        "repository": REPOSITORY,
        "pr_number": pr_number,
        "head_sha": approved_sha,
        "base_sha": main_sha,
        "trusted_sha": main_sha,
        "run_id": run_id,
        "run_attempt": attempt,
        "workflow_path": WORKFLOW,
    }
    require(manifest.keys() == expected.keys(), "Validation manifest has an unsupported schema")
    require(all(type(manifest[key]) is type(value) and manifest[key] == value for key, value in expected.items()), "Manifest does not match the approved PR, current main, or latest run attempt")
    trusted_file = api(f"{prefix}/contents/{WORKFLOW}?ref={manifest['trusted_sha']}")
    main_file = api(f"{prefix}/contents/{WORKFLOW}?ref=main")
    require(trusted_file.get("type") == "file" and trusted_file.get("path") == WORKFLOW and SHA.fullmatch(trusted_file.get("sha", "")), "Trusted workflow source cannot be verified")
    require(trusted_file["sha"] == main_file.get("sha"), "Validation workflow changed since this run")

    fresh_pr = api(f"{prefix}/pulls/{pr_number}")
    fresh_run = api(f"{prefix}/actions/runs/{run_id}")
    require(pr_identity(fresh_pr) == pr_identity(pr), "PR metadata changed during validation")
    require(fresh_run.get("run_attempt") == attempt and fresh_run.get("status") == "completed" and fresh_run.get("conclusion") == "success", "Validation was rerun or changed during verification")
    require(api(f"{prefix}/git/ref/heads/main")["object"]["sha"] == main_sha, "Main changed during verification")
    return latest["target_url"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pr", type=int, required=True)
    parser.add_argument("--approved-sha", required=True)
    args = parser.parse_args()
    print(f"Verified trusted validation: {verify(args.pr, args.approved_sha)}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, zipfile.BadZipFile, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Validation verification failed: {error}") from error
