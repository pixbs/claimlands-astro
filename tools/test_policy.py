"""Regression tests proving publication and artifact gates reject bad input."""
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import policy
import check_artifact


class PolicyTests(unittest.TestCase):
    def test_prohibited_metadata_variants(self):
        for text in ["feat/codex-job", "CLAUDE", "Co-authored-by: KiMi <x@y.z>", "ＣＯＤＥＸ", "prefixclaudesuffix"]:
            with self.subTest(text=text), self.assertRaises(ValueError):
                policy.validate_text(text)

    def test_conventional_messages(self):
        policy.validate_message("feat(world): generate tile topology\n\nFixes #12")
        for text in ["", "update things", "feat: " + "x" * 80, "fix: valid\n\nCo-authored-by: Claude <x@y.z>"]:
            with self.subTest(text=text), self.assertRaises(ValueError):
                policy.validate_message(text)

    def test_branch_names(self):
        for name in ["feat/123-topology", "fix/5-ron-validation"]:
            policy.validate_branch(name)
        for name in ["main", "feat/topology", "feat/0-empty", "feat/123-Upper", "a/b/c", "feat/123-trailing-", "feat/123-codex-tool"]:
            with self.subTest(name=name), self.assertRaises(ValueError):
                policy.validate_branch(name)

    def test_prompt_header_is_first_and_uppercase(self):
        header = (Path(__file__).parents[1] / "AGENTS.md").read_text().splitlines()[0]
        self.assertEqual(header, header.upper())
        self.assertTrue(header.startswith("NEVER INCLUDE "))
        for word in ["CLAUDE", "CODEX", "KIMI", "CO-AUTHOR", "STOP BEFORE PUBLISHING"]:
            self.assertIn(word, header)

    def test_non_tip_commits_and_changed_pr_fail(self):
        pr = {"title": "feat: topology", "body": "tests", "head": {"sha": "abc", "ref": "feat/1-topology"}, "base": {"sha": "base", "ref": "main"}}
        commit = {"commit": {"message": "feat: topology", "author": {"name": "pixbs", "email": "dev@example.test"}, "committer": {"name": "pixbs", "email": "dev@example.test"}}}
        with patch.object(policy, "api", side_effect=[pr, {"object": {"sha": "base"}}, {"commits": [commit], "total_commits": 1}, {**pr, "body": "edited"}]):
            with self.assertRaisesRegex(ValueError, "changed"):
                policy.check_pr(1)
        bad = {"commit": {**commit["commit"], "message": "feat: CODEX"}}
        with patch.object(policy, "api", side_effect=[pr, {"object": {"sha": "base"}}, {"commits": [bad, commit], "total_commits": 2}]):
            with self.assertRaises(ValueError):
                policy.check_pr(1)

    def test_incomplete_artifact_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(ValueError):
                check_artifact.validate(Path(directory))


if __name__ == "__main__":
    unittest.main()
