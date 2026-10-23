# SPDX-License-Identifier: GPL-3.0-or-later
import importlib.util
import os
import plistlib
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
GUARD = ROOT / 'macos/scripts/release-signing.py'


class ReleaseSigningTests(unittest.TestCase):
    def module(self):
        self.assertTrue(GUARD.is_file(), 'Release packaging needs a fail-closed signing guard')
        spec = importlib.util.spec_from_file_location('release_signing', GUARD)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_packaging_checks_signing_and_launches_the_extracted_archive(self):
        package = (ROOT / 'macos/scripts/package-app.sh').read_text()
        self.assertIn('release-signing.py preflight', package)
        self.assertIn('release-signing.py sign', package)
        self.assertIn('release-signing.py archive', package)
        self.assertLess(package.index('release-signing.py preflight'), package.index('build-core.sh\n'))
        self.assertLess(package.index('release-signing.py sign'), package.index('codesign --verify --deep'))
        self.assertLess(package.index('ditto -c -k --norsrc --keepParent "$APP" "$ZIP"'), package.index('release-signing.py archive'))
        app = (ROOT / 'macos/Momentum/App/MomentumApp.swift').read_text()
        self.assertIn('--verify-release-launch', app)
        self.assertLess(app.index('--verify-release-launch'), app.index('Preferences.migrateLegacyDomain()'))

    def test_public_distribution_rejects_ad_hoc_development_and_missing_team(self):
        m = self.module()
        for identity, team in [('-', ''), ('Apple Development: Fixture', 'ABCDEFGHIJ'),
                               ('Apple Distribution: Fixture', 'ABCDEFGHIJ'),
                               ('Developer ID Application: Fixture', '')]:
            with self.subTest(identity=identity):
                with self.assertRaises(ValueError):
                    m.signing_settings(identity, team)
        self.assertEqual(m.signing_settings('Developer ID Application: Fixture', 'ABCDEFGHIJ'),
                         {'identity': 'Developer ID Application: Fixture', 'team': 'ABCDEFGHIJ'})

    def test_deep_codesign_success_alone_cannot_approve_an_ad_hoc_runtime_bundle(self):
        m = self.module()
        with self.assertRaises(ValueError):
            m.validate_metadata('Signature=adhoc\nTeamIdentifier=not set\nCodeDirectory flags=0x10002(adhoc,runtime)',
                                {}, 'ABCDEFGHIJ', require_developer_id=True)

    def test_runtime_and_same_team_required_without_validation_or_debug_exceptions(self):
        m = self.module()
        good = 'Authority=Developer ID Application: Fixture\nTeamIdentifier=ABCDEFGHIJ\nCodeDirectory flags=0x10000(runtime)'
        m.validate_metadata(good, {}, 'ABCDEFGHIJ', require_developer_id=True)
        for metadata, entitlements in [
            (good.replace('ABCDEFGHIJ', 'XXXXXXXXXX'), {}),
            (good.replace('0x10000(runtime)', '0x0(none)'), {}),
            (good, {'com.apple.security.get-task-allow': True}),
            (good, {'com.apple.security.cs.disable-library-validation': True}),
            (good.replace('Developer ID Application:', 'Apple Development:'), {})]:
            with self.subTest(metadata=metadata, entitlements=entitlements):
                with self.assertRaises(ValueError):
                    m.validate_metadata(metadata, entitlements, 'ABCDEFGHIJ', require_developer_id=True)

    def test_non_native_universal_slice_cannot_hide_a_validation_exception(self):
        m = self.module()
        binary = Path('/fixture/Momentum')
        good = b'Authority=Developer ID Application: Fixture\nTeamIdentifier=ABCDEFGHIJ\nCodeDirectory flags=0x10000(runtime)'
        def tool(args):
            if args[0] == 'lipo':
                return subprocess.CompletedProcess(args, 0, b'arm64 x86_64', b'')
            if '--entitlements' in args:
                ent = {'com.apple.security.cs.disable-library-validation': True} if 'x86_64' in args else {}
                return subprocess.CompletedProcess(args, 0, plistlib.dumps(ent), b'')
            return subprocess.CompletedProcess(args, 0, b'', good)
        with patch.object(m, 'code_objects', return_value=([binary], [])), patch.object(m, 'checked', side_effect=tool):
            with self.assertRaises(ValueError):
                m.verify_bundle(Path('/fixture'), 'ABCDEFGHIJ')

    def test_nested_code_discovery_is_inside_out_and_rejects_external_links(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as name:
            app = (Path(name) / 'Fixture.app').resolve()
            library = app / 'Contents/Frameworks/Probe.framework/Versions/B/Probe'
            main = app / 'Contents/MacOS/Fixture'
            for path in [library, main]:
                path.parent.mkdir(parents=True, exist_ok=True);path.write_bytes(bytes.fromhex('cffaedfe') + b'fixture')
            (library.parent.parent / 'Current').symlink_to('B')
            resource = app / 'Contents/resource.txt';resource.write_text('ordinary data')
            binaries, bundles = m.code_objects(app)
            self.assertEqual(binaries, [library, main])
            self.assertEqual(bundles[-1], app)
            self.assertNotIn(resource, binaries)
            (app / 'external').symlink_to('/private/tmp')
            with self.assertRaises(ValueError):
                m.code_objects(app)

    def test_archive_path_and_symlink_escape_fail_before_extraction(self):
        m = self.module()
        self.assertTrue(hasattr(m, 'validate_archive_paths'), 'Validate archive links before ditto can extract them')
        with tempfile.TemporaryDirectory() as name:
            archive = Path(name) / 'fixture.zip'
            for member, target in [('../outside', None), ('Momentum.app/link', '../../../outside')]:
                with zipfile.ZipFile(archive, 'w') as zipped:
                    if target:
                        item = zipfile.ZipInfo(member);item.create_system = 3;item.external_attr = 0o120777 << 16
                        zipped.writestr(item, target)
                    else:
                        zipped.writestr(member, 'fixture')
                with zipfile.ZipFile(archive) as zipped, self.assertRaises(ValueError):
                    m.validate_archive_paths(zipped)

    def test_notarization_cannot_be_omitted_from_distribution_preflight(self):
        m = self.module()
        self.assertTrue(hasattr(m, 'notarization_settings'), 'Distribution preflight must require notarization')
        with self.assertRaises(ValueError):
            m.notarization_settings({})
        with tempfile.TemporaryDirectory() as name:
            key = Path(name) / 'fixture.p8';key.write_text('not a real private key')
            config = {'APPLE_NOTARY_KEY_FILE': str(key), 'APPLE_NOTARY_KEY_ID': 'ABCDEFGHIJ',
                      'APPLE_NOTARY_ISSUER_ID': '00000000-0000-0000-0000-000000000001'}
            m.notarization_settings(config)
            for item in config:
                with self.subTest(item=item), self.assertRaises(ValueError):
                    m.notarization_settings({**config, item: ''})

    def test_missing_distribution_credentials_fail_without_disclosing_values(self):
        m = self.module()
        self.assertTrue(hasattr(m, 'release_credentials'), 'Public signing credentials must be required')
        required = ['MACOS_CERTIFICATE_P12', 'MACOS_CERTIFICATE_PASSWORD', 'APPLE_TEAM_ID',
                    'APPLE_NOTARY_KEY', 'APPLE_NOTARY_KEY_ID', 'APPLE_NOTARY_ISSUER_ID']
        config = {name: 'fixture-private-value' for name in required}
        m.release_credentials(config)
        for name in required:
            with self.subTest(name=name), self.assertRaises(ValueError) as error:
                m.release_credentials({**config, name: ''})
            self.assertNotIn('fixture-private-value', str(error.exception))

    @unittest.skipUnless(sys.platform == 'darwin', 'Requires native macOS dyld/codesign')
    def test_real_hardened_adhoc_framework_crash_and_consistent_identity_launch(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as temp:
            folder = Path(temp)
            library = folder / 'libprobe.dylib';binary = folder / 'probe'
            (folder / 'library.c').write_text('int probe(void) { return 42; }\n')
            (folder / 'main.c').write_text('#include <stdio.h>\nextern int probe(void);\n'
                                          'int main(void) { if (probe() != 42) return 1; '
                                          'puts("Momentum release launch verified"); return 0; }\n')
            def run(*args):
                result = subprocess.run(args, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
            run('xcrun', 'clang', '-dynamiclib', str(folder / 'library.c'), '-o', str(library),
                '-Wl,-install_name,@rpath/libprobe.dylib')
            run('xcrun', 'clang', str(folder / 'main.c'), '-o', str(binary), '-L' + str(folder),
                '-lprobe', '-Wl,-rpath,@executable_path')
            for path in [library, binary]:
                run('codesign', '--force', '--options', 'runtime', '--timestamp=none', '--sign', '-', str(path))
            # Both strict signature checks pass in the shipped failure case.
            for path in [library, binary]:
                run('codesign', '--verify', '--strict', str(path))
            with self.assertRaises(ValueError):
                m.verify_launch(binary)
            identity = os.environ.get('MOMENTUM_SIGNING_FIXTURE_IDENTITY')
            if not identity:
                if os.environ.get('MOMENTUM_REQUIRE_SIGNING_FIXTURE') == '1':
                    self.fail('Distribution CI must exercise a real certificate-signed launch')
                self.skipTest('Negative dyld case passed; certificate-signed half needs an explicit test identity')
            for path in [library, binary]:
                run('codesign', '--force', '--options', 'runtime', '--timestamp=none', '--sign', identity, str(path))
            m.verify_launch(binary)

    def test_launch_probe_has_private_state_and_no_inherited_credentials(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as temp:
            binary = Path(temp) / 'fixture'
            binary.write_text('#!' + sys.executable + '\nimport os,pathlib\n'
                              'assert "ANTHROPIC_API_KEY" not in os.environ\n'
                              'assert os.environ["HOME"] != os.path.expanduser("~") or "release-launch" in os.environ["HOME"]\n'
                              'assert "--verify-release-launch" in __import__("sys").argv\n'
                              'print("Momentum release launch verified")\n')
            binary.chmod(0o700)
            with patch.dict(os.environ, {'ANTHROPIC_API_KEY': 'fake-regression-value'}):
                m.verify_launch(binary)

    def test_launch_rejects_dyld_abort_wrong_marker_and_timeout(self):
        m = self.module()
        with tempfile.TemporaryDirectory() as temp:
            binary = Path(temp) / 'fixture'
            for body in ['raise SystemExit(134)', 'print("wrong marker")', 'import time;time.sleep(5)']:
                binary.write_text('#!' + sys.executable + '\n' + body + '\n');binary.chmod(0o700)
                with self.subTest(body=body), self.assertRaises(ValueError):
                    m.verify_launch(binary, timeout=0.1)


if __name__ == '__main__':
    unittest.main()
