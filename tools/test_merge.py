"""The owner merge path must reject candidate, stale, and dirty checkouts."""
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch
import merge


class MergeTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(__file__).resolve().parents[1]
        self.main, self.head = "a" * 40, "b" * 40
        self.checkout, self.branch, self.dirty = self.main, "", ""
        self.executed, self.loaded, self.verified = [], [], []
        self.remote_calls = 0
        self.change_main = False
        self.change_pr = False
        self.policy_calls = 0

    def api(self, path):
        self.assertEqual(path, f"repos/{merge.REPOSITORY}/git/ref/heads/main")
        self.remote_calls += 1
        return {"object": {"sha": "c" * 40 if self.change_main and self.remote_calls > 1 else self.main}}

    def capture(self, root, *args):
        self.assertEqual(root, self.root)
        if args == ("git", "rev-parse", "--show-toplevel"):
            return str(self.root) + "\n"
        if args == ("git", "branch", "--show-current"):
            return self.branch + "\n"
        if args == ("git", "rev-parse", "HEAD"):
            return self.checkout + "\n"
        if args == ("git", "-c", "core.fsmonitor=false", "status", "--porcelain=v1", "--untracked-files=all", "--ignore-submodules=none"):
            return self.dirty
        if args == ("git", "show", f"{self.main}:tools/merge.py"):
            return (self.root / "tools" / "merge.py").read_text(encoding="utf-8")
        self.fail(f"Unexpected command: {args}")

    def execute(self, root, *args):
        self.assertEqual(root, self.root)
        self.executed.append(args)

    def load(self, root, revision, name):
        self.assertEqual((root, revision), (self.root, self.main))
        self.loaded.append(name)

        def policy(pr):
            self.assertEqual(pr, 22)
            self.policy_calls += 1
            return "d" * 40 if self.change_pr and self.policy_calls > 1 else self.head

        def validation(pr, sha):
            self.verified.append((pr, sha))
            return "https://example.test/validation"

        return SimpleNamespace(check_pr=policy) if name == "policy" else SimpleNamespace(verify=validation)

    def invoke(self):
        merge.merge_approved(22, self.head, self.root, self.api, self.capture, self.execute, self.load)

    def test_candidate_checkout_is_rejected_before_validators_or_merge(self):
        self.branch = "feat/22-foundation"
        with self.assertRaisesRegex(ValueError, "Candidate"):
            self.invoke()
        self.assertEqual((self.loaded, self.executed), ([], []))

    def test_stale_main_is_rejected_before_validators_or_merge(self):
        self.checkout = "e" * 40
        with self.assertRaisesRegex(ValueError, "stale"):
            self.invoke()
        self.assertEqual((self.loaded, self.executed), ([], []))

    def test_tracked_and_untracked_changes_are_rejected_before_validators_or_merge(self):
        for status in [" M tools/policy.py\n", "?? tools/extra.py\n", "M  tools/verify_validation.py\n"]:
            with self.subTest(status=status):
                self.dirty = status
                with self.assertRaisesRegex(ValueError, "dirty"):
                    self.invoke()
                self.assertEqual((self.loaded, self.executed), ([], []))

    def test_trusted_checks_precede_merge_with_exact_approved_head(self):
        self.invoke()
        self.assertEqual(self.loaded, ["policy", "verify_validation"])
        self.assertEqual(self.verified, [(22, self.head)])
        self.assertEqual(self.policy_calls, 2)
        self.assertEqual(self.remote_calls, 2)
        self.assertEqual(self.executed, [
            ("gh", "pr", "checks", "22", "--repo", merge.REPOSITORY, "--required"),
            ("gh", "pr", "merge", "22", "--repo", merge.REPOSITORY, "--rebase", "--match-head-commit", self.head),
        ])

    def test_modified_entry_point_is_rejected_even_if_status_is_clean(self):
        def capture(root, *args):
            if args == ("git", "show", f"{self.main}:tools/merge.py"):
                return "# Trusted source differs from the working copy\n"
            return self.capture(root, *args)

        with self.assertRaisesRegex(ValueError, "entry point differs"):
            merge.merge_approved(22, self.head, self.root, self.api, capture, self.execute, self.load)
        self.assertEqual((self.loaded, self.executed), ([], []))

    def test_changed_main_or_pr_never_reaches_merge(self):
        for setting in ["change_main", "change_pr"]:
            with self.subTest(setting=setting):
                self.setUp()
                setattr(self, setting, True)
                with self.assertRaises(ValueError):
                    self.invoke()
                self.assertFalse(any(command[:3] == ("gh", "pr", "merge") for command in self.executed))

    def test_unisolated_python_is_rejected(self):
        with patch.object(merge.sys, "flags", SimpleNamespace(isolated=0)):
            with self.assertRaisesRegex(ValueError, "python -I"):
                merge.main()

    def test_module_loading_uses_verified_git_source_without_import_path_or_bytecode(self):
        with patch.object(merge, "read", return_value="ORIGIN = 'verified-git-source'\n") as capture:
            module = merge.load_trusted_module(self.root, self.main, "policy")
        capture.assert_called_once_with(self.root, "git", "show", f"{self.main}:tools/policy.py")
        self.assertEqual(module.ORIGIN, "verified-git-source")


if __name__ == "__main__":
    unittest.main()
