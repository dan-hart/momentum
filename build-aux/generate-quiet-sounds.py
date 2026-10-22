#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Reproduce original quiet resonator sounds; no external samples or dependencies."""
from pathlib import Path
import hashlib
import json
import math
import struct
import wave

ROOT = Path(__file__).resolve().parents[1]
RATE = 44100
CHOICES = [
    ('soft-knock', .22, [(0, 520, .065, 1)], 'wood'),
    ('little-lift', .32, [(0, 440, .10, .8), (.075, 660, .12, 1)], 'felt'),
    ('felt-finish', .28, [(0, 523.25, .12, 1), (0, 659.25, .11, .35)], 'felt'),
    ('warm-thumb', .30, [(0, 392, .13, 1), (.06, 587.33, .10, .45)], 'pluck'),
    ('gentle-pair', .30, [(0, 580, .07, .75), (.095, 730, .075, 1)], 'wood'),
    ('small-bloom', .40, [(0, 349.23, .15, .55), (.045, 440, .14, .65), (.09, 523.25, .16, .8)], 'felt'),
    ('sunny-tap', .24, [(0, 698.46, .075, 1), (0, 1046.5, .055, .15)], 'wood'),
    ('pocket-harp', .37, [(0, 440, .13, .6), (.05, 554.37, .12, .65), (.10, 659.25, .13, .7)], 'pluck'),
    ('easy-landing', .36, [(0, 659.25, .10, .5), (.08, 523.25, .15, .85)], 'felt'),
    ('quiet-cheer', .43, [(0, 392, .12, .55), (.07, 493.88, .12, .6), (.14, 587.33, .13, .7)], 'pluck'),
]
RATIOS = {
    'wood': [(1, 1), (2.76, .18), (5.4, .035)],
    'felt': [(1, 1), (2, .19), (3, .035)],
    'pluck': [(1, 1), (2, .25), (3, .10), (4, .025)],
}


def render(duration, notes, kind):
    samples = [0.0] * int(duration * RATE)
    for onset, freq, decay, gain in notes:
        for i in range(int(onset * RATE), len(samples)):
            t = i / RATE - onset
            envelope = (1 - math.exp(-t / .0025)) * math.exp(-t / decay) * gain
            samples[i] += envelope * sum(
                amp * math.sin(2 * math.pi * freq * ratio * t) * math.exp(-t * (ratio - 1) * 8)
                for ratio, amp in RATIOS[kind]
            )
    for i in range(len(samples)):
        samples[i] *= min(1, (len(samples) - 1 - i) / (.025 * RATE))
    rms = math.sqrt(sum(x * x for x in samples) / len(samples))
    peak = max(map(abs, samples))
    gain = min(10 ** (-32 / 20) / rms, 10 ** (-18 / 20) / peak)
    return [x * gain for x in samples]


def main():
    metadata = []
    directories = [ROOT / 'data/resources/sounds',
                   ROOT / 'macos/Packages/MomentumKit/Sources/MomentumKit/Resources',
                   ROOT / 'android/app/src/main/res/raw']
    for name, duration, notes, kind in CHOICES:
        samples = render(duration, notes, kind)
        pcm = struct.pack('<' + 'h' * len(samples), *[round(x * 32767) for x in samples])
        for directory in directories:
            filename = name.replace('-', '_') if directory.name == 'raw' else name
            with wave.open(str(directory / (filename + '.wav')), 'wb') as audio:
                audio.setparams((1, 2, RATE, 0, 'NONE', 'not compressed'))
                audio.writeframes(pcm)
        master = directories[0] / (name + '.wav')
        metadata.append(dict(slug=name, duration=duration, kind=kind, notes=notes,
                             sha256=hashlib.sha256(master.read_bytes()).hexdigest()))
    (directories[0] / 'quiet-provenance.json').write_text(json.dumps(dict(
        date='2026-10-22', creator='Momentum contributors', license='GPL-3.0-or-later',
        origin='Original additive resonator synthesis; no third-party samples',
        generator='build-aux/generate-quiet-sounds.py', default='little-lift',
        peak_ceiling_dbfs=-18, rms_ceiling_dbfs=-32, options=metadata), indent=2) + '\n')


if __name__ == '__main__':
    main()
