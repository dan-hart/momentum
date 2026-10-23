#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Fail-closed Sparkle archive signing and channel publication validation.

Uses only Python stdlib; cryptographic verification uses Apple's CryptoKit on the
macOS release runner. Signing uses the checksum-pinned official Sparkle tool.
Keys are never command-line arguments, files, or subprocess diagnostics.
"""
import argparse
import base64
import hashlib
import json
import os
import plistlib
import zipfile
from pathlib import Path
import re
import subprocess
import sys
import time
import urllib.request
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]
NS = 'http://www.andymatuschak.org/xml-namespaces/sparkle'
S = '{' + NS + '}'
ET.register_namespace('sparkle', NS)
VERSION = re.compile(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\Z')
BUILD = re.compile(r'[1-9][0-9]*\Z')
BASE = 'https://github.com/dan-hart/momentum/releases'


def decoded(value, size, label):
    try:
        raw = base64.b64decode(value, validate=True)
    except (ValueError, TypeError):
        raise ValueError('Invalid ' + label) from None
    if len(raw) != size:
        raise ValueError('Invalid ' + label)
    return raw


def validate_public_key(value):
    decoded(value, 32, 'SPARKLE_PUBLIC_KEY')


def crypto(payload):
    result = subprocess.run(['swift', str(ROOT / 'build-aux/sparkle-crypto.swift')],
                            input=json.dumps(payload), capture_output=True, text=True, timeout=120)
    if result.returncode:
        raise ValueError('Sparkle cryptographic verification failed')


def credentials(shape_only=False):
    public = os.environ.get('SPARKLE_PUBLIC_KEY', '').strip()
    private = os.environ.get('SPARKLE_PRIVATE_KEY', '').strip()
    validate_public_key(public)
    decoded(private, 32, 'SPARKLE_PRIVATE_KEY seed')
    if not shape_only:
        crypto({'publicKey': public, 'privateKey': private})
    print('Sparkle signing configuration verified')


def verify_signature(archive, public, signature):
    validate_public_key(public)
    decoded(signature, 64, 'archive signature')
    crypto({'publicKey': public, 'archive': str(archive.resolve()), 'signature': signature})


def parse_version(version):
    if not VERSION.fullmatch(version):
        raise ValueError('Expected X.Y.Z release version')
    return tuple(map(int, version.split('.')))


def write_feed(archive, output, version, build, signature):
    parse_version(version)
    if not BUILD.fullmatch(build):
        raise ValueError('Invalid release build')
    decoded(signature, 64, 'archive signature')
    if archive.name != f'Momentum-v{version}-macos.zip' or archive.is_symlink() or not archive.is_file():
        raise ValueError('Archive does not match release version')
    rss = ET.Element('rss', {'version': '2.0'})
    channel = ET.SubElement(rss, 'channel')
    ET.SubElement(channel, 'title').text = 'Momentum'
    ET.SubElement(channel, 'link').text = BASE
    item = ET.SubElement(channel, 'item')
    ET.SubElement(item, 'title').text = 'Momentum ' + version
    ET.SubElement(item, 'link').text = BASE + '/tag/v' + version
    ET.SubElement(item, S + 'version').text = build
    ET.SubElement(item, S + 'shortVersionString').text = version
    ET.SubElement(item, S + 'minimumSystemVersion').text = '26.0.0'
    ET.SubElement(item, 'description').text = 'See the release notes for changes in Momentum ' + version + '.'
    ET.SubElement(item, 'enclosure', {'url': BASE + '/download/v' + version + '/' + archive.name,
                                    S + 'edSignature': signature, 'length': str(archive.stat().st_size),
                                    'type': 'application/octet-stream'})
    ET.indent(rss)
    output.write_bytes(ET.tostring(rss, encoding='utf-8', xml_declaration=True))


def read_feed(path):
    if path.is_symlink() or path.stat().st_size > 128 * 1024:
        raise ValueError('Invalid appcast file')
    raw = path.read_bytes()
    if b'<!DOCTYPE' in raw.upper() or b'<!ENTITY' in raw.upper():
        raise ValueError('Appcast entities are not allowed')
    root = ET.fromstring(raw)
    items = root.findall('./channel/item')
    if root.tag != 'rss' or len(items) != 1:
        raise ValueError('Expected exactly one appcast item')
    item = items[0]
    version = item.findtext(S + 'shortVersionString') or ''
    build = item.findtext(S + 'version') or ''
    parse_version(version)
    if not BUILD.fullmatch(build):
        raise ValueError('Invalid appcast build')
    enclosures = item.findall('enclosure')
    if len(enclosures) != 1:
        raise ValueError('Expected exactly one archive')
    return version, int(build), enclosures[0].attrib


def validate_feed(feed, archive, public, previous=None):
    validate_public_key(public)
    version, build, enclosure = read_feed(feed)
    name = f'Momentum-v{version}-macos.zip'
    if archive.name != name or not archive.is_file() or archive.is_symlink():
        raise ValueError('Appcast and archive version differ')
    if enclosure.get('url') != BASE + '/download/v' + version + '/' + name:
        raise ValueError('Untrusted archive URL')
    if enclosure.get('length') != str(archive.stat().st_size):
        raise ValueError('Archive size mismatch')
    verify_signature(archive, public, enclosure.get(S + 'edSignature', ''))
    validate_archive_metadata(archive, public, version, build)
    if previous and previous.exists():
        old_version, old_build, old_enclosure = read_feed(previous)
        if parse_version(old_version) > parse_version(version) or old_build > build:
            raise ValueError('Refusing appcast downgrade')
        if (old_version == version or old_build == build) and (old_version, old_build, old_enclosure) != (version, build, enclosure):
            raise ValueError('Conflicting existing release')


def download_feed():
    request = urllib.request.Request('https://dan-hart.github.io/momentum/appcast.xml', headers={'Cache-Control': 'no-cache'})
    with urllib.request.urlopen(request, timeout=30) as response:
        content = response.read(128 * 1024 + 1)
    if len(content) > 128 * 1024:
        raise ValueError('Hosted feed exceeds maximum size')
    return content


def verify_hosted(feed, attempts=8, delay=20):
    expected = feed.read_bytes()
    for attempt in range(attempts):
        try:
            if download_feed() == expected:
                print('Hosted appcast matches the authenticated public release')
                return
        except (OSError, ValueError):
            pass
        if attempt + 1 < attempts:
            time.sleep(delay)
    raise ValueError('Hosted appcast did not converge to the authenticated release')


def validate_immutable_assets(incoming, existing):
    # A rebuilt zip can differ while keeping its version. Never replace bytes an
    # already-published appcast may authenticate, including partial-run recovery.
    for old in existing.iterdir():
        if old.is_symlink() or not old.is_file():
            raise ValueError('Invalid existing release asset')
        new = incoming / old.name
        if new.exists() and (new.is_symlink() or hashlib.sha256(new.read_bytes()).digest() != hashlib.sha256(old.read_bytes()).digest()):
            raise ValueError('Refusing replacement of an immutable release asset')


def check_public_key(keyfile, previous, public):
    validate_public_key(public)
    if keyfile.is_symlink() or keyfile.read_text().strip() != public:
        raise ValueError('Published key does not match trusted configuration')
    if previous.exists() and (previous.is_symlink() or previous.read_text().strip() != public):
        raise ValueError('Refusing implicit updater signing key rotation')


def validate_archive_metadata(archive, public, version, build):
    try:
        with zipfile.ZipFile(archive) as zipped:
            entries = [entry for entry in zipped.infolist() if entry.filename == 'Momentum.app/Contents/Info.plist']
            if len(entries) != 1 or entries[0].file_size > 1024 * 1024:
                raise ValueError('Missing or duplicate app metadata')
            info = plistlib.loads(zipped.read(entries[0]))
        if (info.get('CFBundleShortVersionString') != version or info.get('CFBundleVersion') != str(build)
                or info.get('SUPublicEDKey') != public or info.get('MomentumDistributionChannel') != 'direct'):
            raise ValueError('Appcast differs from embedded app version, channel or key')
    except (zipfile.BadZipFile, plistlib.InvalidFileException):
        raise ValueError('Invalid archive metadata') from None


def sign(archive, output, version, build, tool):
    credentials()
    result = subprocess.run([str(tool), '--ed-key-file', '-', '-p', str(archive)],
                            input=os.environ['SPARKLE_PRIVATE_KEY'].strip(), capture_output=True, text=True, timeout=120)
    if result.returncode:
        raise ValueError('Sparkle signing tool failed')
    signature = result.stdout.strip()
    verify_signature(archive, os.environ['SPARKLE_PUBLIC_KEY'].strip(), signature)
    write_feed(archive, output, version, build, signature)
    validate_feed(output, archive, os.environ['SPARKLE_PUBLIC_KEY'].strip())
    print('Signed archive and validated appcast')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    sub.add_parser('credentials').add_argument('--shape-only', action='store_true')
    signer = sub.add_parser('sign')
    for cmd in [signer, sub.add_parser('validate')]:
        cmd.add_argument('--archive', required=True, type=Path)
        cmd.add_argument('--feed', required=True, type=Path)
    signer.add_argument('--version', required=True)
    signer.add_argument('--build', required=True)
    signer.add_argument('--tool', required=True, type=Path)
    sub.choices['validate'].add_argument('--previous', type=Path)
    hosted = sub.add_parser('verify-hosted')
    hosted.add_argument('--feed', type=Path, required=True)
    immutable = sub.add_parser('immutable-assets')
    immutable.add_argument('--incoming', type=Path, required=True)
    immutable.add_argument('--existing', type=Path, required=True)
    keycheck = sub.add_parser('check-public-key')
    keycheck.add_argument('--key-file', type=Path, required=True)
    keycheck.add_argument('--previous-key-file', type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.command == 'credentials':
            credentials(args.shape_only)
        elif args.command == 'verify-hosted':
            verify_hosted(args.feed)
        elif args.command == 'immutable-assets':
            validate_immutable_assets(args.incoming, args.existing)
            print('Existing public release assets are immutable')
        elif args.command == 'check-public-key':
            check_public_key(args.key_file, args.previous_key_file, os.environ.get('SPARKLE_PUBLIC_KEY', '').strip())
            print('Updater public key continuity verified')
        elif args.command == 'sign':
            sign(args.archive, args.feed, args.version, args.build, args.tool)
        else:
            validate_feed(args.feed, args.archive, os.environ.get('SPARKLE_PUBLIC_KEY', '').strip(), args.previous)
            print('Public archive signature and appcast continuity verified')
    except (ValueError, OSError, ET.ParseError, subprocess.SubprocessError):
        # Do not echo tool stderr or key material in CI logs.
        print('Sparkle release validation failed; check keys, archive and appcast configuration', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
