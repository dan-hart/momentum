# SPDX-License-Identifier: GPL-3.0-or-later
import base64
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'release-keychain.py'


class ReleaseKeychainTests(unittest.TestCase):
    def fixture(self, folder, fail=False):
        tool = folder / 'security'
        tool.write_text('#!' + sys.executable + '\n' + '''import json,os,pathlib,sys
args=sys.argv[1:];root=pathlib.Path(os.environ['FIXTURE_ROOT'])
if args[:3]==['list-keychains','-d','user']:
 if '-s' in args:
  (root/'search-list.json').write_text(json.dumps(args[4:]))
  if os.environ.get('FIXTURE_RESTORE_FAIL')=='1' and args[4:]==['original.keychain-db']:sys.exit(23)
 else:print('"original.keychain-db"')
elif args[0]=='create-keychain':pathlib.Path(args[-1]).write_text('fixture keychain')
elif args[0]=='delete-keychain':
 if os.environ.get('FIXTURE_DELETE_FAIL')=='1':sys.exit(24)
 pathlib.Path(args[-1]).unlink()
elif args[0]=='import' and os.environ.get('FIXTURE_FAIL')=='1':sys.exit(19)
elif args[0]=='find-identity':print('1) fixture "Developer ID Application: Fixture (ABCDEFGHIJ)"')
''')
        tool.chmod(0o700)
        return {'PATH': str(folder) + ':/usr/bin:/bin', 'RUNNER_TEMP': str(folder),
                'FIXTURE_ROOT': str(folder), 'FIXTURE_FAIL': '1' if fail else '0',
                'MACOS_CERTIFICATE_P12': base64.b64encode(b'fake-p12').decode(),
                'MACOS_CERTIFICATE_PASSWORD': 'fake-p12-password', 'APPLE_TEAM_ID': 'ABCDEFGHIJ',
                'APPLE_NOTARY_KEY': 'fake-private-notary-value', 'GITHUB_ENV': str(folder / 'github-env')}

    def test_import_failure_restores_search_list_and_removes_private_staging(self):
        with tempfile.TemporaryDirectory() as name:
            folder = Path(name);env = self.fixture(folder, fail=True)
            result = subprocess.run([sys.executable, str(SCRIPT), 'create'], env=env, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertFalse((folder / 'momentum-release-signing').exists())
            self.assertEqual(json.loads((folder / 'search-list.json').read_text()), ['original.keychain-db'])
            self.assertNotIn('fake-p12-password', result.stdout + result.stderr)
            self.assertNotIn('fake-private-notary-value', result.stdout + result.stderr)

    def test_cleanup_removes_private_files_even_if_a_system_cleanup_action_fails(self):
        for fault in ['FIXTURE_RESTORE_FAIL', 'FIXTURE_DELETE_FAIL']:
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as name:
                folder = Path(name);env = self.fixture(folder)
                result = subprocess.run([sys.executable, str(SCRIPT), 'create'], env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                result = subprocess.run([sys.executable, str(SCRIPT), 'cleanup'],
                                        env={**env, fault: '1'}, capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((folder / 'momentum-release-signing').exists())
                self.assertNotIn('fake-private-notary-value', result.stdout + result.stderr)

    def test_success_exports_only_public_identity_and_private_file_path_then_cleans_up(self):
        with tempfile.TemporaryDirectory() as name:
            folder = Path(name);env = self.fixture(folder)
            result = subprocess.run([sys.executable, str(SCRIPT), 'create'], env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            stage = folder / 'momentum-release-signing'
            self.assertEqual((stage / 'notary.p8').stat().st_mode & 0o777, 0o600)
            self.assertFalse((stage / 'identity.p12').exists())
            exported = (folder / 'github-env').read_text()
            self.assertIn('MACOS_SIGN_IDENTITY=Developer ID Application: Fixture (ABCDEFGHIJ)', exported)
            self.assertNotIn('fake-private-notary-value', exported)
            result = subprocess.run([sys.executable, str(SCRIPT), 'cleanup'], env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(stage.exists())
            self.assertEqual(json.loads((folder / 'search-list.json').read_text()), ['original.keychain-db'])


if __name__ == '__main__':
    unittest.main()
