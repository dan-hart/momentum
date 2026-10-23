#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Run build tools without inheriting unrelated shell/application credentials.

Explicit toolchain configuration survives. Unlisted variables never reach the
child, including future providers' credentials. No credential-name denylist,
secret-value filtering, login shell, or environment-value diagnostics are used.
"""
import os
from pathlib import Path
import sys

# Keep this list explicit and review additions. Signing passwords/private keys and
# account tokens belong to the release operation that needs them, never Xcode.
BUILD_VARIABLES = frozenset({
    'HOME', 'USER', 'LOGNAME', 'PATH', 'TMPDIR', 'LANG', 'LC_ALL', 'LC_CTYPE',
    'LC_MESSAGES', 'TZ', 'SHELL', 'DEVELOPER_DIR', 'SDKROOT',
    'CARGO_HOME', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN', 'CARGO_TARGET_DIR',
    'CARGO_PROFILE_RELEASE_STRIP', 'MACOSX_DEPLOYMENT_TARGET',
    'IPHONEOS_DEPLOYMENT_TARGET',
    'MOMENTUM_UPDATE_PUBLIC_KEY', 'MOMENTUM_DISTRIBUTION_CHANNEL',
    'CI', 'TERM', 'NO_COLOR', 'MOMENTUM_TEST_NEARBY',
})


def build_environment(source):
    return {name: value for name, value in source.items() if name in BUILD_VARIABLES}


def build_command(arguments):
    command = list(arguments)
    if command and command[0] == '--':
        command.pop(0)
    if not command:
        raise ValueError('A build command is required')
    if Path(command[0]).name == 'xcodebuild' and '-hideShellScriptEnvironment' not in command:
        command.insert(1, '-hideShellScriptEnvironment')
    return command


def main():
    try:
        command = build_command(sys.argv[1:])
        os.execvpe(command[0], command, build_environment(os.environ))
    except (OSError, ValueError):
        # Do not format argv/environment: either can include user-supplied secrets.
        print('Build launcher could not start the requested tool', file=sys.stderr)
        return 127


if __name__ == '__main__':
    sys.exit(main())
