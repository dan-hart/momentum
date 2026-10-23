# SPDX-License-Identifier: GPL-3.0-or-later
"""No real credentials: sentinels prove the build process boundary drops secrets."""
import json
import os
import shutil
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
LAUNCHER = ROOT / 'macos/scripts/build-env.py'


class BuildEnvironmentTests(unittest.TestCase):
    def invoke(self, command, values=None):
        environment = {'HOME': str(Path.home()), 'PATH': os.defpath,
                       'USER': 'fixture', 'LANG': 'en_US.UTF-8',
                       'ANTHROPIC_API_KEY': 'fixture-not-a-credential',
                       'UNKNOWN_PRIVATE_VALUE': 'another-fixture',
                       'SPARKLE_PRIVATE_KEY': 'not-a-real-signing-key',
                       'FUTURE_PROVIDER_TOKEN': 'never-forward-this'}
        environment.update(values or {})
        return subprocess.run([sys.executable, str(LAUNCHER), *command], env=environment,
                              capture_output=True, text=True)

    def test_build_child_never_receives_unrelated_or_future_secret_variables(self):
        result = self.invoke([sys.executable, '-c', 'import os,json; print(json.dumps(dict(os.environ)))'])
        self.assertEqual(result.returncode, 0, result.stderr)
        received = json.loads(result.stdout)
        for key in ['ANTHROPIC_API_KEY', 'UNKNOWN_PRIVATE_VALUE', 'SPARKLE_PRIVATE_KEY', 'FUTURE_PROVIDER_TOKEN']:
            self.assertNotIn(key, received)
        self.assertEqual(received['USER'], 'fixture')
        self.assertNotIn('fixture-not-a-credential', result.stdout + result.stderr)

    def test_needed_toolchain_paths_and_public_configuration_survive(self):
        result = self.invoke([sys.executable, '-c', 'import os,json; print(json.dumps(dict(os.environ)))'],
                             {'DEVELOPER_DIR': '/fixture/Xcode', 'CARGO_HOME': '/fixture/cargo',
                              'RUSTUP_HOME': '/fixture/rustup', 'TMPDIR': '/tmp',
                              'MOMENTUM_APPLE_BUILD_LOCKED': '1', 'MOMENTUM_UPDATE_PUBLIC_KEY': 'public-value', 'MOMENTUM_TEST_NEARBY': '1'})
        self.assertEqual(result.returncode, 0, result.stderr)
        received = json.loads(result.stdout)
        self.assertEqual(received['DEVELOPER_DIR'], '/fixture/Xcode')
        self.assertEqual(received['CARGO_HOME'], '/fixture/cargo')
        self.assertNotIn('MOMENTUM_APPLE_BUILD_LOCKED', received)
        self.assertEqual(received['MOMENTUM_UPDATE_PUBLIC_KEY'], 'public-value')
        self.assertEqual(received.get('MOMENTUM_TEST_NEARBY'), '1')

    def test_xcode_logging_is_disabled_and_arguments_remain_literal(self):
        with tempfile.TemporaryDirectory() as name:
            tool = Path(name) / 'xcodebuild'
            tool.write_text('#!' + sys.executable + '\nimport json,sys; print(json.dumps(sys.argv[1:]))\n')
            tool.chmod(0o700)
            result = self.invoke([str(tool), '-scheme', 'name with spaces;$(literal)', 'build'])
        self.assertEqual(result.returncode, 0, result.stderr)
        args = json.loads(result.stdout)
        self.assertIn('-hideShellScriptEnvironment', args)
        self.assertIn('name with spaces;$(literal)', args)
        self.assertEqual(args.count('-hideShellScriptEnvironment'), 1)

    def test_exit_status_and_missing_command_are_reported_without_environment_dump(self):
        result = self.invoke([sys.executable, '-c', 'raise SystemExit(27)'])
        self.assertEqual(result.returncode, 27)
        result = self.invoke(['/nonexistent/fixture-build-tool'])
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('fixture-not-a-credential', result.stdout + result.stderr)

    def test_inherited_lock_marker_cannot_skip_core_builder_isolation(self):
        with tempfile.TemporaryDirectory() as name:
            root = Path(name)
            scripts = root / 'macos/scripts'
            scripts.mkdir(parents=True)
            for filename in ['build-core.sh', 'build-core-impl.sh', 'build-env.py']:
                source = ROOT / 'macos/scripts' / filename
                if source.exists(): shutil.copyfile(source, scripts / filename)
            tools = root / 'tools'
            tools.mkdir()
            code = {
                'rustc': "print('host: aarch64-apple-darwin')",
                'rustup': "print('aarch64-apple-darwin')",
                'cargo': "import os; print('credential-present=' + str('UNRELATED_TOKEN' in os.environ)); raise SystemExit(55)",
            }
            for tool, body in code.items():
                path = tools / tool
                path.write_text('#!' + sys.executable + '\n' + body + '\n')
                path.chmod(0o700)
            environment = {'HOME': str(root), 'PATH': str(tools) + os.pathsep + os.defpath,
                           'MOMENTUM_APPLE_BUILD_LOCKED': '1', 'UNRELATED_TOKEN': 'harmless-fixture'}
            result = subprocess.run(['/bin/sh', str(scripts / 'build-core.sh'), '--debug'], env=environment,
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 55, result.stderr)
            self.assertIn('credential-present=False', result.stdout)

    def test_apple_project_script_phases_disable_environment_printing(self):
        # No YAML dependency: each named target script's settings block must opt out.
        for filename in ['macos/project.yml', 'ios/project.yml']:
            source = (ROOT / filename).read_text()
            for block in source.split('basedOnDependencyAnalysis: false')[:-1]:
                self.assertIn('showEnvVars: false', block.split('- name:')[-1], filename)


if __name__ == '__main__':
    unittest.main()
