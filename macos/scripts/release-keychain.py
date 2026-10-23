#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Import distribution secrets into a temporary release keychain; never log values."""
import base64
import json
import os
from pathlib import Path
import re
import secrets
import shlex
import shutil
import subprocess
import sys


def run(*arguments):
    result = subprocess.run(arguments, capture_output=True, text=True, timeout=120)
    if result.returncode:
        raise ValueError('Release keychain operation failed')
    return result.stdout


def cleanup(root):
    receipt = root / 'receipt.json'
    if not receipt.exists():
        return
    previous = json.loads(receipt.read_text())
    failed = False
    try:
        run('security', 'list-keychains', '-d', 'user', '-s', *previous)
    except (ValueError, OSError, subprocess.SubprocessError):
        failed = True
    keychain = root / 'release.keychain-db'
    try:
        if keychain.exists():
            run('security', 'delete-keychain', str(keychain))
    except (ValueError, OSError, subprocess.SubprocessError):
        failed = True
    finally:
        shutil.rmtree(root)
    if failed:
        raise ValueError('Private staging removed, but system keychain cleanup needs inspection')


def create(root):
    root.mkdir(mode=0o700)
    previous = shlex.split(run('security', 'list-keychains', '-d', 'user'))
    (root / 'receipt.json').write_text(json.dumps(previous))
    (root / 'receipt.json').chmod(0o600)
    try:
        keychain = root / 'release.keychain-db'
        password = secrets.token_hex(32)
        p12 = root / 'identity.p12'
        p12.write_bytes(base64.b64decode(os.environ['MACOS_CERTIFICATE_P12'], validate=True))
        p12.chmod(0o600)
        notary = root / 'notary.p8'
        notary.write_text(os.environ['APPLE_NOTARY_KEY']);notary.chmod(0o600)
        run('security', 'create-keychain', '-p', password, str(keychain))
        run('security', 'set-keychain-settings', '-lut', '21600', str(keychain))
        run('security', 'unlock-keychain', '-p', password, str(keychain))
        run('security', 'import', str(p12), '-P', os.environ['MACOS_CERTIFICATE_PASSWORD'],
            '-T', '/usr/bin/codesign', '-T', '/usr/bin/security', '-k', str(keychain))
        run('security', 'set-key-partition-list', '-S', 'apple-tool:,apple:,codesign:', '-s', '-k', password, str(keychain))
        p12.unlink()
        run('security', 'list-keychains', '-d', 'user', '-s', str(keychain), *previous)
        identities = run('security', 'find-identity', '-v', '-p', 'codesigning', str(keychain))
        matches = re.findall(r'"(Developer ID Application: [^"\r\n]+)"', identities)
        if len(matches) != 1 or not matches[0].endswith('(' + os.environ['APPLE_TEAM_ID'] + ')'):
            raise ValueError('Expected one matching Developer ID identity')
        with Path(os.environ['GITHUB_ENV']).open('a') as output:
            output.write('MACOS_SIGN_IDENTITY=' + matches[0] + '\n')
            output.write('APPLE_NOTARY_KEY_FILE=' + str(notary) + '\n')
    except Exception:
        cleanup(root)
        raise


def main():
    try:
        root = Path(os.environ['RUNNER_TEMP']) / 'momentum-release-signing'
        if sys.argv[1:] == ['cleanup']:
            cleanup(root)
        elif sys.argv[1:] == ['create']:
            create(root)
        else:
            raise ValueError('Expected create or cleanup')
        print('Temporary release signing keychain:', sys.argv[1])
    except (ValueError, OSError, KeyError, subprocess.SubprocessError):
        print('Temporary release signing keychain operation failed', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
