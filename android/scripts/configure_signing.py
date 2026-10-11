#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Prepare an optional CI signing key without printing secret values."""
import base64
import os
import uuid
from pathlib import Path

NAMES = ("ANDROID_KEYSTORE_BASE64", "ANDROID_KEYSTORE_PASSWORD", "ANDROID_KEY_ALIAS", "ANDROID_KEY_PASSWORD")


def configure(values, temporary, environment_file):
    supplied = [values.get(name, "") for name in NAMES]
    if not any(supplied):
        return False
    if not all(value and value.strip() for value in supplied):
        raise ValueError("Android signing requires all four ANDROID_KEYSTORE_* / ANDROID_KEY_* secrets")
    try:
        key = base64.b64decode(supplied[0], validate=True)
    except ValueError as error:
        raise ValueError("Android signing key must be valid base64") from error
    if not key:
        raise ValueError("Android signing key is empty")
    key_path = Path(temporary) / f"momentum-android-key-{uuid.uuid4().hex}.jks"
    descriptor = os.open(key_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "wb") as output:
            output.write(key)
        with Path(environment_file).open("a") as output:
            for name, value in zip(("MOMENTUM_KEYSTORE", "MOMENTUM_KEYSTORE_PASSWORD", "MOMENTUM_KEY_ALIAS", "MOMENTUM_KEY_PASSWORD"),
                                   (str(key_path), *supplied[1:])):
                delimiter = "MOMENTUM_" + uuid.uuid4().hex
                output.write(f"{name}<<{delimiter}\n{value}\n{delimiter}\n")
    except BaseException:
        key_path.unlink(missing_ok=True)
        raise
    return True


if __name__ == "__main__":
    try:
        signed = configure(os.environ, os.environ["RUNNER_TEMP"], os.environ["GITHUB_ENV"])
        print("Android release signing configured" if signed else "Android release will contain explicitly unsigned artifacts")
    except (ValueError, OSError) as error:
        raise SystemExit(str(error))
