#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Public macOS distribution signing and isolated runtime acceptance gates."""
import argparse
import os
from pathlib import Path
import plistlib
import posixpath
import re
import subprocess
import tempfile
import zipfile
import uuid

MARKER = 'Momentum release launch verified'
MACHO = {bytes.fromhex(s) for s in ('feedface', 'cefaedfe', 'feedfacf', 'cffaedfe', 'cafebabe', 'bebafeca', 'cafebabf', 'bfbafeca')}


def release_credentials(environment):
    required = ('MACOS_CERTIFICATE_P12', 'MACOS_CERTIFICATE_PASSWORD', 'APPLE_TEAM_ID',
                'APPLE_NOTARY_KEY', 'APPLE_NOTARY_KEY_ID', 'APPLE_NOTARY_ISSUER_ID')
    if any(not environment.get(name, '').strip() for name in required):
        raise ValueError('Public macOS release signing/notarization configuration is incomplete')


def notarization_settings(environment):
    path = environment.get('APPLE_NOTARY_KEY_FILE', '')
    key_id = environment.get('APPLE_NOTARY_KEY_ID', '')
    issuer = environment.get('APPLE_NOTARY_ISSUER_ID', '')
    if not path or not Path(path).is_file() or not re.fullmatch(r'[A-Z0-9]+', key_id):
        raise ValueError('Public macOS releases require complete notarization inputs')
    try:
        uuid.UUID(issuer)
    except (ValueError, AttributeError):
        raise ValueError('Public macOS releases require a notarization issuer') from None


def signing_settings(identity, team):
    if not identity.startswith('Developer ID Application:') or not re.fullmatch(r'[A-Z0-9]{10}', team):
        raise ValueError('Public macOS releases require a Developer ID Application identity and matching team')
    return {'identity': identity, 'team': team}


def validate_metadata(metadata, entitlements, team, require_developer_id=True):
    actual = re.search(r'^TeamIdentifier=(.+)$', metadata, re.M)
    flags = re.search(r'flags=0x([0-9a-fA-F]+)', metadata)
    if not actual or actual[1] != team or not flags or not (int(flags[1], 16) & 0x10000):
        raise ValueError('Release code must have hardened runtime and the expected signing team')
    if require_developer_id and not re.search(r'^Authority=Developer ID Application:', metadata, re.M):
        raise ValueError('Release code requires Developer ID Application signing')
    for key in ('com.apple.security.get-task-allow', 'com.apple.security.cs.disable-library-validation'):
        if entitlements.get(key, False):
            raise ValueError('Release code must retain library validation and exclude debugging entitlements')


def checked(arguments):
    result = subprocess.run([str(a) for a in arguments], capture_output=True, timeout=120)
    if result.returncode:
        # Tool errors may include signing configuration; never echo argv/environment.
        raise ValueError('Release signing tool failed')
    return result


def code_objects(app):
    app = app.resolve()
    binaries, bundles = [], []
    for path in app.rglob('*'):
        if path.is_symlink():
            if not path.resolve().is_relative_to(app):
                raise ValueError('Release bundle contains an external symlink')
            continue
        if path.is_dir() and path.suffix in ('.app', '.xpc', '.framework'):
            bundles.append(path)
        elif path.is_file():
            with path.open('rb') as file:
                if file.read(4) in MACHO:
                    binaries.append(path)
    if not binaries:
        raise ValueError('Release bundle contains no executable code')
    return sorted(binaries, key=lambda p: len(p.parts), reverse=True), sorted(bundles, key=lambda p: len(p.parts), reverse=True) + [app]


def sign_bundle(app, identity, team):
    signing_settings(identity, team)
    binaries, bundles = code_objects(app)
    for path in binaries + bundles:
        checked(['codesign', '--force', '--options', 'runtime', '--timestamp', '--preserve-metadata=identifier,entitlements', '--sign', identity, path])
    verify_bundle(app, team)


def verify_bundle(app, team):
    binaries, bundles = code_objects(app)
    for path in binaries:
        checked(['codesign', '--verify', '--strict', path])
        architectures = checked(['lipo', '-archs', path]).stdout.decode().split()
        if not architectures:
            raise ValueError('Release executable has no supported architecture')
        for architecture in architectures:
            metadata = checked(['codesign', '-d', '--architecture', architecture, '--verbose=4', path]).stderr.decode()
            raw = checked(['codesign', '-d', '--architecture', architecture, '--entitlements', ':-', path]).stdout
            entitlements = plistlib.loads(raw) if raw.strip() else {}
            validate_metadata(metadata, entitlements, team)
    for path in bundles:
        checked(['codesign', '--verify', '--strict', path])
    checked(['codesign', '--verify', '--deep', '--strict', app])


def verify_launch(binary, timeout=15):
    with tempfile.TemporaryDirectory(prefix='momentum-release-launch-') as directory:
        root = Path(directory)
        temporary = root / 'tmp';temporary.mkdir()
        # This exact launch exits before preferences, stores, workers or permissions.
        env = {'PATH': '/usr/bin:/bin', 'HOME': str(root), 'CFFIXED_USER_HOME': str(root), 'TMPDIR': str(temporary)}
        try:
            with tempfile.TemporaryFile() as output:
                result = subprocess.run([str(binary), '--verify-release-launch'], env=env,
                                        stdout=output, stderr=output, timeout=timeout)
                output.seek(0);message = output.read(4096).decode(errors='replace')
        except (OSError, subprocess.TimeoutExpired):
            raise ValueError('Packaged app failed the isolated runtime launch check') from None
        if result.returncode or message.strip() != MARKER:
            raise ValueError('Packaged app failed the isolated runtime launch check')


def validate_archive_paths(zipped):
    for item in zipped.infolist():
        path = Path(item.filename)
        if path.is_absolute() or '..' in path.parts:
            raise ValueError('Unsafe release archive paths')
        if (item.external_attr >> 16) & 0o170000 == 0o120000:
            target = zipped.read(item).decode('utf-8')
            normalized = posixpath.normpath(str(path.parent / target))
            if target.startswith('/') or normalized == '..' or normalized.startswith('../'):
                raise ValueError('Unsafe release archive symlink')
    if sum(item.file_size for item in zipped.infolist()) > 2 * 1024 ** 3:
        raise ValueError('Oversized release archive')


def verify_archive(archive, team):
    with zipfile.ZipFile(archive) as zipped:
        validate_archive_paths(zipped)
    with tempfile.TemporaryDirectory(prefix='momentum-release-archive-') as directory:
        checked(['ditto', '-x', '-k', archive, directory])
        app = Path(directory) / 'Momentum.app'
        if not app.is_dir():
            raise ValueError('Release archive does not contain Momentum.app')
        verify_bundle(app, team)
        verify_launch(app / 'Contents/MacOS/Momentum')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['credentials', 'preflight', 'sign', 'verify', 'launch', 'archive'])
    parser.add_argument('--app', type=Path)
    parser.add_argument('--archive', type=Path)
    args = parser.parse_args()
    try:
        identity = os.environ.get('MACOS_SIGN_IDENTITY', '')
        team = os.environ.get('APPLE_TEAM_ID', '')
        if args.command == 'credentials':
            release_credentials(os.environ)
        elif args.command == 'preflight':
            signing_settings(identity, team)
            notarization_settings(os.environ)
        elif args.command == 'archive':
            verify_archive(args.archive, team)
        elif args.command == 'launch':
            verify_launch(args.app / 'Contents/MacOS/Momentum')
        elif args.command == 'sign':
            sign_bundle(args.app, identity, team)
        else:
            verify_bundle(args.app, team)
        print('macOS release signing gate passed:', args.command)
    except (ValueError, OSError, subprocess.SubprocessError, plistlib.InvalidFileException, zipfile.BadZipFile):
        print('macOS release signing/launch gate failed; public distribution refused', file=__import__('sys').stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
