#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
# Fails when a source file with translatable strings is missing from po/POTFILES.in,
# so new dialogs cannot silently skip the translators.
set -eu
cd "$(dirname "$0")/.."
missing=0
for f in $(grep -rl --include='*.rs' 'gettext(' crates/app/src) \
         $(grep -rl --include='*.blp' '_("' data/resources/ui) \
         data/*.desktop.in.in data/*.metainfo.xml.in.in data/*.gschema.xml.in; do
  if ! grep -qxF "$f" po/POTFILES.in; then
    echo "missing from po/POTFILES.in: $f"
    missing=1
  fi
done
for f in $(grep -v '^#' po/POTFILES.in); do
  [ -f "$f" ] || { echo "listed in po/POTFILES.in but gone: $f"; missing=1; }
done
[ "$missing" = 0 ] && echo "po/POTFILES.in is complete"
exit $missing
