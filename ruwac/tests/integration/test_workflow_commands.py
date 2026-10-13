#!/usr/bin/env python3
"""Execute the exact YAML-selected CI test script; reproduce folded-command faults."""
import os
import re
from pathlib import Path
import subprocess
import unittest
import yaml


class WorkflowCommands(unittest.TestCase):
    def script(self):
        self.root = Path(__file__).resolve().parents[3]
        workflow = yaml.safe_load((self.root / '.github/workflows/quality.yml').read_text())
        step = next(step for step in workflow['jobs']['quality']['steps']
                    if step.get('name') == 'Testing artifact attestation subject regressions')
        self.assertIsInstance(step['run'], str)
        return step['run']

    def execute(self, script):
        env = {k: v for k, v in os.environ.items() if k not in {'BASH_ENV', 'ENV'}}
        return subprocess.run(['bash', '-eu', '-o', 'pipefail', '-c', script],
                              cwd=self.root, env=env, capture_output=True, text=True, timeout=30)

    def test_actual_workflow_step_executes_both_suites(self):
        result = self.execute(self.script())
        self.assertEqual(result.returncode, 0, (result.stdout + result.stderr)[-2500:])
        self.assertEqual(len(re.findall(r'Ran \d+ tests', result.stderr)), 2)
        self.assertEqual(result.stderr.count('\nOK\n'), 2)

    def test_previous_folded_form_is_rejected(self):
        script = self.script()
        result = self.execute(' '.join(script.splitlines()))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('FAILED', result.stderr)


if __name__ == '__main__':
    unittest.main()
