# SPDX-License-Identifier: GPL-3.0-or-later
import base64
import importlib.util
import os
import plistlib
import zipfile
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET
import yaml

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('sparkle_release', ROOT / 'build-aux/sparkle-release.py')
M = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(M)


class FeedTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name)
        self.archive = self.folder / 'Momentum-v0.4.14-macos.zip'
        self.feed = self.folder / 'appcast.xml'
        self.public = base64.b64encode(bytes(range(32))).decode()
        self.signature = base64.b64encode(bytes(range(64))).decode()
        with zipfile.ZipFile(self.archive, 'w') as archive:
            archive.writestr('Momentum.app/Contents/Info.plist', plistlib.dumps({
                'CFBundleShortVersionString': '0.4.14', 'CFBundleVersion': '414',
                'SUPublicEDKey': self.public, 'MomentumDistributionChannel': 'direct'}))

    def create(self):
        M.write_feed(self.archive, self.feed, '0.4.14', '414', self.signature)

    def test_feed_binds_version_build_archive_length_and_download_origin(self):
        self.create()
        with patch.object(M, 'verify_signature') as verify:
            M.validate_feed(self.feed, self.archive, self.public)
        verify.assert_called_once_with(self.archive, self.public, self.signature)
        root = ET.parse(self.feed).getroot()
        self.assertEqual(root.findtext('./channel/item/' + M.S + 'version'), '414')
        self.assertEqual(root.find('./channel/item/enclosure').attrib['length'], str(self.archive.stat().st_size))

    def test_malformed_versions_builds_and_signature_fail_before_output(self):
        for version, build, signature in [('0.4.14-beta', '414', self.signature), ('0.4.14', '0', self.signature),
                                           ('0.4.14', '414\nattack', self.signature), ('0.4.14', '414', 'bad')]:
            with self.subTest(version=version, build=build):
                with self.assertRaises(ValueError):
                    M.write_feed(self.archive, self.feed, version, build, signature)
                self.assertFalse(self.feed.exists())

    def test_tampered_url_size_version_or_extra_item_are_rejected(self):
        self.create()
        original = self.feed.read_text()
        for tampered in [original.replace('https://github.com', 'https://evil.invalid'),
                         original.replace('length="' + str(self.archive.stat().st_size) + '"', 'length="1"'),
                         original.replace('0.4.14</', '0.4.13</'),
                         original.replace('</channel>', '<item/></channel>')]:
            self.feed.write_text(tampered)
            with patch.object(M, 'verify_signature'):
                with self.assertRaises(ValueError):
                    M.validate_feed(self.feed, self.archive, self.public)

    def test_older_or_conflicting_same_version_never_replaces_previous_feed(self):
        self.create()
        previous = self.folder / 'previous.xml'
        previous.write_bytes(self.feed.read_bytes())
        with patch.object(M, 'verify_signature'):
            M.validate_feed(self.feed, self.archive, self.public, previous)
            previous.write_text(previous.read_text().replace('>414<', '>415<'))
            with self.assertRaises(ValueError):
                M.validate_feed(self.feed, self.archive, self.public, previous)
        self.assertIn('>415<', previous.read_text())

    def test_archive_corruption_or_untrusted_key_fails_verification(self):
        self.create()
        self.archive.write_bytes(b'other archive')
        with self.assertRaises(ValueError):
            M.validate_feed(self.feed, self.archive, self.public)
        with self.assertRaises(ValueError):
            M.validate_public_key('not a key')

    def test_matching_size_corruption_still_requires_crypto_verification(self):
        self.create()
        self.archive.write_bytes(b'x' * self.archive.stat().st_size)
        with patch.object(M, 'verify_signature', side_effect=ValueError('invalid signature')):
            with self.assertRaises(ValueError):
                M.validate_feed(self.feed, self.archive, self.public)

    def test_key_preflight_does_not_disclose_secret(self):
        secret = 'private-not-base64-fixture'
        result = subprocess.run([sys.executable, str(ROOT / 'build-aux/sparkle-release.py'), 'credentials'],
                                env={**os.environ, 'SPARKLE_PRIVATE_KEY': secret, 'SPARKLE_PUBLIC_KEY': self.public},
                                capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn(secret, result.stdout + result.stderr)

    def test_feed_cannot_change_public_key_continuity(self):
        self.create()
        previous = self.folder / 'previous.xml'
        previous.write_bytes(self.feed.read_bytes())
        with patch.object(M, 'verify_signature', side_effect=ValueError('key mismatch')):
            with self.assertRaises(ValueError):
                M.validate_feed(self.feed, self.archive, self.public, previous)

    def test_public_key_cannot_rotate_by_changing_release_variable(self):
        keyfile = self.folder / 'key.txt'
        previous = self.folder / 'previous-key.txt'
        keyfile.write_text(self.public)
        previous.write_text(self.public)
        M.check_public_key(keyfile, previous, self.public)
        previous.write_text(base64.b64encode(bytes(32)).decode())
        with self.assertRaises(ValueError):
            M.check_public_key(keyfile, previous, self.public)

    def test_embedded_archive_version_build_and_key_are_required(self):
        self.create()
        for key, value in [('CFBundleVersion', '415'), ('CFBundleShortVersionString', '0.4.15'),
                           ('SUPublicEDKey', base64.b64encode(bytes(32)).decode()),
                           ('MomentumDistributionChannel', 'source')]:
            with self.subTest(key=key):
                with zipfile.ZipFile(self.archive, 'w') as archive:
                    info = {'CFBundleShortVersionString': '0.4.14', 'CFBundleVersion': '414',
                            'SUPublicEDKey': self.public, 'MomentumDistributionChannel': 'direct'}
                    info[key] = value
                    archive.writestr('Momentum.app/Contents/Info.plist', plistlib.dumps(info))
                with patch.object(M, 'verify_signature'):
                    with self.assertRaises(ValueError):
                        M.validate_feed(self.feed, self.archive, self.public)

    def test_release_rerun_never_overwrites_existing_asset_bytes(self):
        incoming = self.folder / 'incoming'
        existing = self.folder / 'existing'
        incoming.mkdir(); existing.mkdir()
        (incoming / 'Momentum.zip').write_bytes(b'new build')
        (existing / 'Momentum.zip').write_bytes(b'old advertised build')
        with self.assertRaises(ValueError):
            M.validate_immutable_assets(incoming, existing)
        self.assertEqual((existing / 'Momentum.zip').read_bytes(), b'old advertised build')
        (incoming / 'Momentum.zip').write_bytes(b'old advertised build')
        M.validate_immutable_assets(incoming, existing)
        (incoming / 'new-missing-artifact.txt').write_bytes(b'allowed addition')
        M.validate_immutable_assets(incoming, existing)

    def test_hosted_readback_requires_exact_feed_bytes_and_retries_old_content(self):
        self.create()
        with patch.object(M, 'download_feed', side_effect=[b'old feed', self.feed.read_bytes()]), patch.object(M.time, 'sleep'):
            M.verify_hosted(self.feed, attempts=2, delay=0)
        with patch.object(M, 'download_feed', return_value=b'old feed'), patch.object(M.time, 'sleep'):
            with self.assertRaises(ValueError):
                M.verify_hosted(self.feed, attempts=2, delay=0)


class WorkflowTests(unittest.TestCase):
    def test_existing_asset_guard_precedes_any_release_upload(self):
        workflow = yaml.safe_load((ROOT / '.github/workflows/release.yml').read_text())
        steps = workflow['jobs']['release']['steps']
        guard = next(i for i, step in enumerate(steps) if 'immutable-assets' in step.get('run', ''))
        publish = next(i for i, step in enumerate(steps) if step.get('uses', '').startswith('softprops/action-gh-release'))
        self.assertLess(guard, publish)
        self.assertIn('(HTTP 404)', steps[guard]['run'])
        self.assertNotIn('|| true', steps[guard]['run'])

    def test_feed_publication_waits_for_public_assets_and_gates_final_deploy(self):
        workflow = yaml.safe_load((ROOT / '.github/workflows/release.yml').read_text())
        jobs = workflow['jobs']
        self.assertIn('release', jobs['sparkle-feed']['needs'])
        self.assertIn('flatpak-repo', jobs['sparkle-feed']['needs'])
        self.assertIn('sparkle-feed', jobs['install-channels']['needs'])
        steps = jobs['sparkle-feed']['steps']
        validation = next(i for i, step in enumerate(steps) if 'check-public-key' in step.get('run', ''))
        publish = next(i for i, step in enumerate(steps) if 'publish-git' in step.get('run', ''))
        self.assertLess(validation, publish)
        self.assertIn('--path appcast.xml --path sparkle-public-key.txt', steps[publish]['run'])
        self.assertIn('verify-hosted', steps[-1]['run'])

    def test_final_archive_is_signed_after_packaging_and_crypto_tests_are_required(self):
        workflow = yaml.safe_load((ROOT / '.github/workflows/release.yml').read_text())
        steps = workflow['jobs']['macos']['steps']
        package = next(i for i, step in enumerate(steps) if 'run' in step and 'package-app.sh' in step['run'])
        sign = next(i for i, step in enumerate(steps) if 'sparkle-release.py sign' in step.get('run', ''))
        self.assertLess(package, sign)
        self.assertTrue(any('SPARKLE_SIGN_TOOL' in step.get('env', {}) for step in steps))


@unittest.skipUnless(os.environ.get('SPARKLE_SIGN_TOOL'), 'set SPARKLE_SIGN_TOOL to test real Sparkle/CryptoKit interoperability')
class CryptoIntegrationTests(unittest.TestCase):
    def test_real_signing_tamper_wrong_key_and_secret_mismatch(self):
        # RFC 8032 public test vector, never a production identity.
        seed = base64.b64encode(bytes.fromhex('9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60')).decode()
        public = base64.b64encode(bytes.fromhex('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a')).decode()
        with tempfile.TemporaryDirectory() as name:
            folder = Path(name)
            archive = folder / 'Momentum-v0.4.14-macos.zip'
            with zipfile.ZipFile(archive, 'w') as file:
                file.writestr('Momentum.app/Contents/Info.plist', plistlib.dumps({
                    'CFBundleShortVersionString': '0.4.14', 'CFBundleVersion': '414',
                    'SUPublicEDKey': public, 'MomentumDistributionChannel': 'direct'}))
            feed = folder / 'appcast.xml'
            with patch.dict(os.environ, SPARKLE_PRIVATE_KEY=seed, SPARKLE_PUBLIC_KEY=public):
                M.sign(archive, feed, '0.4.14', '414', Path(os.environ['SPARKLE_SIGN_TOOL']))
                M.validate_feed(feed, archive, public)
                archive.write_bytes(b'X' * archive.stat().st_size)
                with self.assertRaises(ValueError):
                    M.validate_feed(feed, archive, public)
                with patch.dict(os.environ, SPARKLE_PUBLIC_KEY=base64.b64encode(bytes(32)).decode()):
                    with self.assertRaises(ValueError):
                        M.credentials()


if __name__ == '__main__':
    unittest.main()
