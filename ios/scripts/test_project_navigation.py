#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 Dan Hart
"""Opt-in real-touch B-142 acceptance on an explicitly disposable iPhone simulator.

Requires existing Xcode/simctl and AXe; no XCUITest or Python dependencies.
Launches only --demo data, leaves the simulator installed, and never erases stores.
For AXe on this host use a process-scoped Xcode 26.5 DEVELOPER_DIR.
The compact toolbar is occasionally omitted from AXe. The documented fallback
is limited to a 402 x 874 point iPhone 17 Pro fixture and the visible leading
Show lists control. All project taps use the current accessibility row label.
"""
import argparse
import json
import plistlib
import subprocess
import sys
import time
from pathlib import Path


def run(command):
    result = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=30)
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip() or str(command))
    return result.stdout


def nodes(value):
    if isinstance(value, dict):
        yield value
        for child in value.get('children', []):
            yield from nodes(child)
    elif isinstance(value, list):
        for child in value:
            yield from nodes(child)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--udid', required=True)
    parser.add_argument('--app', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--confirm-disposable', action='store_true', required=True,
                        help='Confirm this explicit simulator contains no user task data.')
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    devices = json.loads(run(['xcrun', 'simctl', 'list', 'devices', '-j']))['devices']
    matches = [(runtime, device) for runtime, group in devices.items() for device in group
               if device['udid'] == args.udid]
    if len(matches) != 1:
        raise RuntimeError('The explicit simulator was not found.')
    runtime, device = matches[0]
    if not device.get('isAvailable') or device['state'] != 'Booted':
        raise RuntimeError('Boot the explicit disposable simulator first.')
    if 'iPhone-17-Pro' != device.get('deviceTypeIdentifier', '').split('.')[-1]:
        raise RuntimeError('The toolbar fallback is scoped to an iPhone 17 Pro fixture.')
    info = plistlib.loads((args.app / 'Info.plist').read_bytes())
    bundle = info['CFBundleIdentifier']
    summary = {'device': device['name'], 'udid': args.udid, 'runtime': runtime,
               'bundle': bundle, 'version': info.get('CFBundleShortVersionString'),
               'build': info.get('CFBundleVersion'), 'app': str(args.app.resolve()),
               'mode': '--demo', 'steps': []}
    current = None

    def capture(name):
        nonlocal current
        current = json.loads(run(['axe', 'describe-ui', '--udid', args.udid]))
        (args.output / (name + '.json')).write_text(json.dumps(current, indent=2) + '\n')
        return list(nodes(current))

    def wait(name, predicate):
        deadline = time.monotonic() + 6
        while time.monotonic() < deadline:
            elements = capture(name)
            if predicate(elements):
                return elements
            time.sleep(0.1)
        raise RuntimeError('Timed out at ' + name + '; see the captured accessibility tree.')

    def label_set(elements):
        return {item.get('AXLabel') for item in elements}

    def ready(name, project, expected):
        elements = wait(name, lambda items:
            any(item.get('AXUniqueId') == project for item in items)
            and expected.issubset(label_set(items))
            and 'Loading tasks' not in label_set(items)
            and 'Loading Momentum' not in label_set(items))
        summary['steps'].append({'name': name, 'selected': project,
            'taskControls': len([item for item in elements if (item.get('AXLabel') or '').startswith('Open ')])})
        (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')

    def tap(*options):
        # Allow the native column slide to finish before tapping the next row.
        run(['axe', 'tap', *options, '--udid', args.udid, '--post-delay', '0.45'])

    def lists(name):
        elements = capture(name + '-before')
        if 'Show lists' in label_set(elements):
            tap('--label', 'Show lists')
        else:
            application = next((item for item in elements if item.get('type') == 'Application'), {})
            frame = application.get('frame', {})
            if frame.get('width') != 402 or frame.get('height') != 874:
                raise RuntimeError('Expected the documented compact iPhone viewport for toolbar fallback.')
            tap('-x', '38', '-y', '85')
        elements = wait(name, lambda items: 'Lists' in label_set(items) and 'Projects' in label_set(items))
        return elements

    def project(name, title, expected):
        elements = lists(name + '-lists')
        rows = [item['AXLabel'] for item in elements
                if (item.get('AXLabel') or '').startswith(title + ', ') and item.get('type') == 'Button']
        if len(rows) != 1:
            # SwiftUI exposes project links as Link on some runtimes.
            rows = [item['AXLabel'] for item in elements if (item.get('AXLabel') or '').startswith(title + ', ')]
        if len(rows) != 1:
            raise RuntimeError('Expected exactly one current project row for ' + title)
        tap('--label', rows[0])
        ready(name, title, expected)

    try:
        run(['xcrun', 'simctl', 'install', args.udid, str(args.app)])
        run(['xcrun', 'simctl', 'launch', '--terminate-running-process', args.udid, bundle, '--demo', '-AppleLanguages', '(en)', '-AppleLocale', 'en_US'])
        today = {'Open Stretch and plan the day', 'Open Review sync conflict handling'}
        home = {'Open Dentist appointment', 'Open Renew library books'}
        momentum = {'Open Review sync conflict handling', 'Open Write release notes for 0.1'}
        ready('initial-today', 'Today', today)
        project('first-home-touch', 'Home', home)
        run(['axe', 'screenshot', '--udid', args.udid, '--output', str(args.output / 'first-home-touch.png')])
        project('home-reentry', 'Home', home)
        project('first-momentum-touch', 'Momentum', momentum)
        lists('return-today-lists')
        tap('--label', 'Today')
        ready('return-today', 'Today', today)
        project('home-after-today', 'Home', home)
        project('momentum-after-home', 'Momentum', momentum)
        summary['outcome'] = 'passed'
        print('PASS: first project touch, selected-project reentry, project switches and Today return.')
        return 0
    except (RuntimeError, OSError, ValueError, subprocess.TimeoutExpired) as error:
        summary['outcome'] = 'failed'
        summary['error'] = str(error)
        print('FAIL: ' + str(error), file=sys.stderr)
        return 1
    finally:
        (args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')


if __name__ == '__main__':
    sys.exit(main())
