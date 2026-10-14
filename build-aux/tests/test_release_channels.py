#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Release configuration and publication against isolated real Git repositories."""
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import yaml

ROOT = Path(__file__).resolve().parents[2]
CHANNELS = ROOT / "build-aux/release-channels.py"
CREDENTIALS = ("HOMEBREW_TAP_SSH_KEY", "FLATPAK_GPG_PRIVATE_KEY", "FLATPAK_GPG_KEY_ID", "FLATPAK_GPG_FINGERPRINT")


def run(*args, cwd=None, env=None):
    return subprocess.run(args, cwd=cwd, env=env, text=True, capture_output=True)


def git(repo, *args):
    result = run("git", "-C", str(repo), *args)
    if result.returncode:
        raise AssertionError(result.stderr)
    return result.stdout.strip()


def assets(directory, version="0.4.11"):
    directory.mkdir()
    names = [f"Momentum-v{version}-macos.zip", f"mo-v{version}-macos-universal.tar.gz",
             f"mo-v{version}-linux-x86_64.tar.gz", f"mo-v{version}-linux-aarch64.tar.gz"]
    for name in names:
        (directory / name).write_bytes((name + "\n").encode())
    (directory / "SHA256SUMS.txt").write_text("".join(
        f"{hashlib.sha256((directory / name).read_bytes()).hexdigest()}  {name}\n" for name in names))
    return names


class CredentialTests(unittest.TestCase):
    def invoke(self, values, channel="all"):
        env = {k: v for k, v in os.environ.items() if k not in CREDENTIALS}
        env.update(values)
        if env.get("FLATPAK_GPG_FINGERPRINT") == "fixture-sensitive-value":
            env["FLATPAK_GPG_FINGERPRINT"] = "1" * 40
        return run(sys.executable, str(CHANNELS), "credentials", "--channel", channel, env=env)

    def test_all_configured_credentials_pass_without_disclosure(self):
        values = {name: "fixture-sensitive-value" for name in CREDENTIALS}
        result = self.invoke(values)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("fixture-sensitive-value", result.stdout + result.stderr)

    def test_each_missing_required_credential_fails_without_disclosure(self):
        for missing in CREDENTIALS:
            with self.subTest(missing=missing):
                values = {name: "fixture-sensitive-value" for name in CREDENTIALS if name != missing}
                result = self.invoke(values)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(missing, result.stderr)
                self.assertNotIn("fixture-sensitive-value", result.stdout + result.stderr)

    def test_empty_and_whitespace_values_are_missing(self):
        result = self.invoke({name: " \n " for name in CREDENTIALS})
        self.assertNotEqual(result.returncode, 0)
        for name in CREDENTIALS:
            self.assertIn(name, result.stderr)

    def test_invalid_trust_fingerprint_fails_preflight_without_disclosure(self):
        values = {name: "fixture-sensitive-value" for name in CREDENTIALS}
        values["FLATPAK_GPG_FINGERPRINT"] = "invalid-fixture-fingerprint"
        result = self.invoke(values)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("FLATPAK_GPG_FINGERPRINT", result.stderr)
        self.assertNotIn("invalid-fixture-fingerprint", result.stdout + result.stderr)

    def test_homebrew_check_does_not_require_flatpak_credentials(self):
        result = self.invoke({"HOMEBREW_TAP_SSH_KEY": "fixture-sensitive-value"}, "homebrew")
        self.assertEqual(result.returncode, 0, result.stderr)


class SigningIdentityTests(unittest.TestCase):
    def invoke(self, primary):
        with tempfile.TemporaryDirectory(prefix="momentum-signing-command-fixture-") as name:
            directory = Path(name)
            tool = directory / "gpg"
            records = f"sec:-:2048:1:fixture:::::::::\nfpr:::::::::{primary}:\n"
            tool.write_text("#!/bin/sh\nprintf '%s' '" + records + "'\n")
            tool.chmod(0o755)
            env = {**os.environ, "PATH": str(directory) + os.pathsep + os.environ["PATH"]}
            return run(sys.executable, str(CHANNELS), "check-signing-key", "--key-id", "fixture",
                       "--fingerprint", "1" * 40, env=env)

    def test_matching_imported_signing_identity_passes(self):
        result = self.invoke("1" * 40)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_different_imported_signing_identity_is_rejected(self):
        result = self.invoke("2" * 40)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not match", result.stderr.lower())


class PublicationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="momentum-release-channel-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.tap = self.root / "tap"
        self.remote = self.root / "remote.git"
        self.dist = self.root / "assets"
        self.names = assets(self.dist)
        self.tap.mkdir()
        self.assertEqual(run("git", "init", "--bare", str(self.remote)).returncode, 0)
        git(self.tap, "init", "-b", "main")
        git(self.tap, "config", "user.name", "Release Fixture")
        git(self.tap, "config", "user.email", "fixture@example.invalid")
        old = self.root / "old-assets"
        assets(old, "0.4.10")
        result = run(sys.executable, str(ROOT / "build-aux/release.py"), "homebrew",
                     "--version", "0.4.10", "--assets", str(old), "--out", str(self.tap))
        self.assertEqual(result.returncode, 0, result.stderr)
        (self.tap / "Formula/unrelated.rb").write_text("# Unrelated fixture package\n")
        git(self.tap, "add", ".")
        git(self.tap, "commit", "-m", "Initial fixture")
        git(self.tap, "remote", "add", "origin", str(self.remote))
        git(self.tap, "push", "-u", "origin", "main")
        self.initial = git(self.tap, "rev-parse", "HEAD")

    def publish(self, version="0.4.11", *extra):
        return run(sys.executable, str(CHANNELS), "homebrew", "--tap", str(self.tap),
                   "--assets", str(self.dist), "--version", version, *extra)

    def remote_head(self):
        return git(self.remote, "rev-parse", "refs/heads/main")

    def test_updates_both_definitions_and_preserves_unrelated_packages(self):
        result = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        head = git(self.tap, "rev-parse", "HEAD")
        self.assertEqual(self.remote_head(), head)
        self.assertNotEqual(head, self.initial)
        self.assertEqual(set(git(self.tap, "diff", "--name-only", self.initial, head).splitlines()),
                         {"Casks/momentum.rb", "Formula/momentum-cli.rb"})
        self.assertIn('version "0.4.11"', (self.tap / "Casks/momentum.rb").read_text())
        self.assertEqual(git(self.tap, "status", "--porcelain"), "")

    def test_rerun_is_successful_without_an_extra_commit(self):
        self.assertEqual(self.publish().returncode, 0)
        head = self.remote_head()
        result = self.publish()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("already current", result.stdout)
        self.assertEqual(self.remote_head(), head)
        self.assertEqual(git(self.tap, "rev-parse", "HEAD"), head)

    def test_damaged_asset_is_rejected_before_modification(self):
        before = (self.tap / "Casks/momentum.rb").read_bytes()
        (self.dist / self.names[0]).write_bytes(b"damaged fixture")
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum", result.stderr.lower())
        self.assertEqual((self.tap / "Casks/momentum.rb").read_bytes(), before)
        self.assertEqual(git(self.tap, "status", "--porcelain"), "")
        self.assertEqual(self.remote_head(), self.initial)

    def test_missing_checksum_manifest_is_rejected(self):
        (self.dist / "SHA256SUMS.txt").unlink()
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SHA256SUMS", result.stderr)
        self.assertEqual(self.remote_head(), self.initial)

    def test_dirty_tap_is_rejected_without_overwriting_changes(self):
        original = b"# Inherited fixture change\n"
        (self.tap / "Casks/momentum.rb").write_bytes(original)
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("dirty", result.stderr.lower())
        self.assertEqual((self.tap / "Casks/momentum.rb").read_bytes(), original)
        self.assertEqual(self.remote_head(), self.initial)

    def test_older_version_cannot_downgrade_the_tap(self):
        self.assertEqual(self.publish().returncode, 0)
        head = self.remote_head()
        result = self.publish("0.4.10")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("older", result.stderr.lower())
        self.assertEqual(self.remote_head(), head)
        self.assertEqual(git(self.tap, "status", "--porcelain"), "")

    def test_invalid_version_is_rejected_before_modification(self):
        result = self.publish("0.4.11\nfixture=invalid")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("version", result.stderr.lower())
        self.assertEqual(self.remote_head(), self.initial)

    def test_commit_failure_is_not_reported_as_success(self):
        hook = self.tap / ".git/hooks/pre-commit"
        hook.write_text("#!/bin/sh\necho 'fixture commit rejected' >&2\nexit 1\n")
        hook.chmod(0o755)
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("fixture commit rejected", result.stderr)
        self.assertEqual(self.remote_head(), self.initial)

    def test_push_failure_is_not_reported_as_success(self):
        hook = self.remote / "hooks/pre-receive"
        hook.write_text("#!/bin/sh\necho 'fixture push rejected' >&2\nexit 1\n")
        hook.chmod(0o755)
        result = self.publish()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("fixture push rejected", result.stderr)
        self.assertEqual(self.remote_head(), self.initial)

    def test_retry_after_failed_push_publishes_the_existing_local_commit(self):
        hook = self.remote / "hooks/pre-receive"
        hook.write_text("#!/bin/sh\nexit 1\n")
        hook.chmod(0o755)
        self.assertNotEqual(self.publish().returncode, 0)
        local = git(self.tap, "rev-parse", "HEAD")
        self.assertNotEqual(local, self.remote_head())
        hook.unlink()
        retry = self.publish()
        self.assertEqual(retry.returncode, 0, retry.stderr)
        self.assertEqual(self.remote_head(), local)
        self.assertEqual(git(self.tap, "rev-parse", "HEAD"), local)

    def test_hosted_git_publication_reruns_without_false_commit_failures(self):
        (self.tap / "summary").write_text("isolated export fixture\n")
        args = (sys.executable, str(CHANNELS), "publish-git", "--repo", str(self.tap),
                "--branch", "main", "--message", "Fixture repository export")
        first = run(*args)
        self.assertEqual(first.returncode, 0, first.stderr)
        head = self.remote_head()
        second = run(*args)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(self.remote_head(), head)


class WorkflowTests(unittest.TestCase):
    def load(self, filename):
        path = ROOT / ".github/workflows" / filename
        self.assertTrue(path.exists(), f"Required channel workflow missing: {filename}")
        return yaml.load(path.read_text(), Loader=yaml.BaseLoader)

    def test_missing_credentials_fail_before_builds_and_channels_cannot_be_skipped(self):
        data = self.load("release.yml")
        jobs = data["jobs"]
        self.assertNotIn("if", jobs["flatpak-repo"])
        steps = jobs["verify"]["steps"]
        credentials = [s for s in steps if "release-channels.py credentials" in s.get("run", "")]
        self.assertEqual(len(credentials), 1)
        for name in CREDENTIALS:
            self.assertIn(name, credentials[0]["env"])
        self.assertIn("flatpak-repo", jobs["release"]["needs"])
        self.assertIn("flatpak-repo", jobs["install-channels"]["needs"])
        self.assertIn("release", jobs["install-channels"]["needs"])
        self.assertNotIn("if", jobs["install-channels"])
        self.assertNotIn("${{", data["concurrency"]["group"])

    def test_flathub_submission_and_archive_are_absent(self):
        data = self.load("release.yml")
        self.assertNotIn("flathub", data["jobs"])
        text = (ROOT / ".github/workflows/release.yml").read_text()
        for removed in ("FLATHUB_TOKEN", "release.py flathub", "flatpak-cargo-generator"):
            self.assertNotIn(removed, text)

    def test_hosted_readback_declares_its_external_command_dependencies(self):
        hosted = self.load("install-channels.yml")["jobs"]["hosted"]
        installs = [step["run"] for step in hosted["steps"]
                    if "apt-get install" in step.get("run", "")]
        self.assertEqual(len(installs), 1)
        for package in ("flatpak", "ostree", "gnupg"):
            self.assertIn(package, installs[0])

    def test_pages_build_is_explicit_and_has_permission_before_readback(self):
        data = self.load("install-channels.yml")
        hosted = data["jobs"]["hosted"]
        self.assertEqual(hosted["permissions"]["pages"], "write")
        requests = [i for i, step in enumerate(hosted["steps"])
                    if 'pages/builds' in step.get("run", "")]
        readback = [i for i, step in enumerate(hosted["steps"])
                    if 'release-channels.py verify-hosted' in step.get("run", "")]
        self.assertEqual(len(requests), 1)
        self.assertLess(requests[0], readback[0])
        self.assertEqual(self.load("release.yml")["permissions"]["pages"], "write")

    def test_shared_workflow_uses_the_scoped_key_and_same_publisher(self):
        data = self.load("install-channels.yml")
        self.assertIn("workflow_call", data["on"])
        self.assertIn("workflow_dispatch", data["on"])
        homebrew = data["jobs"]["homebrew"]
        checkout = [s for s in homebrew["steps"]
                    if s.get("with", {}).get("repository") == "dan-hart/homebrew-tap"]
        self.assertEqual(len(checkout), 1)
        self.assertEqual(checkout[0]["with"]["ssh-key"], "${{ secrets.HOMEBREW_TAP_SSH_KEY }}")
        self.assertTrue(any("release-channels.py homebrew" in s.get("run", "") for s in homebrew["steps"]))
        self.assertTrue(any("release-channels.py verify-hosted" in s.get("run", "")
                            for s in data["jobs"]["hosted"]["steps"]))


if __name__ == "__main__":
    unittest.main()
