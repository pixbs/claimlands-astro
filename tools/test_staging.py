"""Regression cases for candidate configuration overriding trusted validators."""
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import stage_validation as staging


class StagingTests(unittest.TestCase):
    def test_candidate_only_cargo_configuration_and_targets_are_removed(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, trusted = Path(temporary) / "candidate", Path(temporary) / "trusted"
            root.mkdir()
            files = {".cargo/config.toml": '[alias]\nxtask = "run --package xtask --"\n',
                     ".config/nextest.toml": "retries = 0\n",
                     "tools/xtask/Cargo.toml": '[package]\nname = "xtask"\n',
                     "tools/xtask/src/main.rs": "fn main() {}\n", "deny.toml": "trusted\n"}
            for name, data in files.items():
                path = trusted / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(data)
            extras = (".cargo/config", ".config/extra.toml", "tools/xtask/build.rs", "tools/xtask/src/bin/xtask.rs")
            for name in extras:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("candidate override")
            staging.replace_controls(root, trusted)
            self.assertTrue(all(not (root / name).exists() for name in extras))
            for name in files:
                self.assertEqual((root / name).read_bytes(), (trusted / name).read_bytes())

    def test_candidate_link_is_unlinked_without_touching_its_destination(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, trusted, outside = (Path(temporary) / name for name in ("candidate", "trusted", "outside"))
            for directory in (root, trusted, outside):
                directory.mkdir()
            for name in staging.CONTROLS:
                (trusted / name).write_text("trusted")
            sentinel = outside / "preserve.txt"
            sentinel.write_text("must survive")
            link = root / "tools"
            # No OS symlink privilege is needed: assert the exact operations used
            # for a link, including that recursive removal is never selected.
            original = Path.is_symlink
            with patch.object(Path, "is_symlink", lambda path: path == link or original(path)), \
                    patch.object(Path, "unlink") as unlink, patch.object(staging.shutil, "rmtree") as remove:
                staging.replace_controls(root, trusted)
                unlink.assert_called_once_with()
                remove.assert_not_called()
            self.assertEqual(sentinel.read_text(), "must survive")

    def test_archive_rejects_traversal_and_symlinks(self):
        for name, kind in (("../escape", tarfile.REGTYPE), ("tools/link", tarfile.SYMTYPE)):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                data = io.BytesIO()
                with tarfile.open(fileobj=data, mode="w") as bundle:
                    entry = tarfile.TarInfo(name)
                    entry.type = kind
                    bundle.addfile(entry)
                with patch.object(staging.subprocess, "check_output", return_value=data.getvalue()):
                    with self.assertRaises(ValueError):
                        staging.export_revision(Path(temporary), "a" * 40, staging.CONTROLS, Path(temporary))


if __name__ == "__main__":
    unittest.main()
