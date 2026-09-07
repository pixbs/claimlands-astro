"""Prove that forged statuses, stale revisions and skipped jobs cannot authorize merges."""
import copy
import io
import json
import unittest
import zipfile
import verify_validation as validation


def archive(record):
    data = io.BytesIO()
    with zipfile.ZipFile(data, "w") as bundle:
        bundle.writestr("validation-manifest.json", json.dumps(record))
    return data.getvalue()


class ValidationTests(unittest.TestCase):
    def setUp(self):
        self.head, self.base = "a" * 40, "b" * 40
        self.pr = {"head": {"sha": self.head}, "base": {"sha": self.base, "ref": "main", "repo": {"full_name": validation.REPOSITORY}}, "title": "feat: build foundation", "body": "Closes #1", "state": "open", "draft": False}
        self.run = {"workflow_id": 7, "path": validation.WORKFLOW, "event": "pull_request_target", "repository": {"full_name": validation.REPOSITORY}, "head_sha": self.head, "status": "completed", "conclusion": "success", "run_attempt": 1}
        self.jobs = [{"name": name, "run_id": 12, "status": "completed", "conclusion": "success"} for name in sorted(validation.REQUIRED_JOBS)]
        self.manifest = {"schema_version": 1, "repository": validation.REPOSITORY, "pr_number": 1, "head_sha": self.head, "base_sha": self.base, "trusted_sha": self.base, "run_id": 12, "run_attempt": 1, "workflow_path": validation.WORKFLOW}

    def api(self, path):
        route = path.split("?", 1)[0].removeprefix(f"repos/{validation.REPOSITORY}/")
        records = {
            "pulls/1": self.pr,
            "git/ref/heads/main": {"object": {"sha": self.base}},
            f"commits/{self.head}/status": {"statuses": [{"context": "quality", "id": 3, "state": "success", "target_url": f"https://github.com/{validation.REPOSITORY}/actions/runs/12"}]},
            "actions/runs/12": self.run,
            "actions/workflows/validate.yml": {"id": 7, "path": validation.WORKFLOW, "state": "active"},
            "actions/runs/12/attempts/1/jobs": {"total_count": len(self.jobs), "jobs": self.jobs},
            "actions/runs/12/artifacts": {"total_count": 1, "artifacts": [{"name": f"validation-manifest-{self.head}", "expired": False, "id": 9, "workflow_run": {"id": 12, "head_sha": self.head}}]},
            f"contents/{validation.WORKFLOW}": {"type": "file", "path": validation.WORKFLOW, "sha": "c" * 40},
        }
        return copy.deepcopy(records[route])

    def verify(self):
        return validation.verify(1, self.head, self.api, lambda _: archive(self.manifest))

    def test_complete_trusted_run_passes(self):
        self.assertTrue(self.verify().endswith("/12"))

    def test_success_status_from_candidate_workflow_fails(self):
        self.run["event"] = "push"
        with self.assertRaisesRegex(ValueError, "trusted"):
            self.verify()

    def test_skipped_or_missing_jobs_fail(self):
        self.jobs[0]["conclusion"] = "skipped"
        with self.assertRaisesRegex(ValueError, "skipped"):
            self.verify()
        self.jobs.pop(0)
        with self.assertRaisesRegex(ValueError, "missing"):
            self.verify()

    def test_stale_manifest_and_wrong_workflow_fail(self):
        self.manifest["run_attempt"] = 2
        with self.assertRaisesRegex(ValueError, "Manifest"):
            self.verify()
        self.manifest["run_attempt"] = 1
        self.run["workflow_id"] = 8
        with self.assertRaisesRegex(ValueError, "different workflow"):
            self.verify()

    def test_head_change_and_unapproved_schema_fail(self):
        self.pr["head"]["sha"] = self.base
        with self.assertRaisesRegex(ValueError, "head has changed"):
            self.verify()
        self.pr["head"]["sha"] = self.head
        self.manifest["additional"] = True
        with self.assertRaisesRegex(ValueError, "schema"):
            self.verify()
