"""Exercise the actual workflow shell with local outputs and a recording Docker stub."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


WORKFLOW = (Path(__file__).resolve().parents[1] / '.github/workflows/docker-image.yml').read_text()
AMD64 = 'sha256:' + 'a' * 64
ARM64 = 'sha256:' + 'b' * 64


def step_script(name: str) -> str:
    step = WORKFLOW.split(f'      - name: {name}\n', 1)[1].split('\n      - name:', 1)[0]
    block = step.split('        run: |\n', 1)[1]
    lines = []
    for line in block.splitlines():
        if line and not line.startswith('          '):
            break
        lines.append(line[10:])
    return '\n'.join(lines) + '\n'


class DockerWorkflowOutputTests(unittest.TestCase):
    def test_artifact_free_matrix_wiring(self) -> None:
        self.assertIn('DOCKER_BUILD_RECORD_UPLOAD: "false"', WORKFLOW)
        for forbidden in ('actions/upload-artifact', 'actions/download-artifact', '/tmp/digests', 'continue-on-error:'):
            self.assertNotIn(forbidden, WORKFLOW)
        for arch in ('amd64', 'arm64'):
            self.assertIn(f'digest_{arch}: ${{{{ steps.digest.outputs.digest_{arch} }}}}', WORKFLOW)
            self.assertIn(f'{arch.upper()}_DIGEST: ${{{{ needs.publish-platform.outputs.digest_{arch} }}}}', WORKFLOW)
        self.assertIn('ARCH: ${{ matrix.arch }}', WORKFLOW)
        self.assertIn('DIGEST: ${{ steps.build.outputs.digest }}', WORKFLOW)
        self.assertIn('push-by-digest=true,name-canonical=true,push=true', WORKFLOW)

    def test_exports_both_architectures_in_either_completion_order(self) -> None:
        for order in ((('amd64', AMD64), ('arm64', ARM64)), (('arm64', ARM64), ('amd64', AMD64))):
            with self.subTest(order=order), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / 'outputs'
                for arch, digest in order:
                    result = subprocess.run(['bash', '-c', step_script('Export digest')], env={**os.environ, 'ARCH': arch, 'DIGEST': digest, 'GITHUB_OUTPUT': str(output)}, capture_output=True, text=True)
                    self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(dict(line.split('=', 1) for line in output.read_text().splitlines()), {'digest_amd64': AMD64, 'digest_arm64': ARM64})

    def test_export_rejects_missing_malformed_and_injected_values(self) -> None:
        for arch, digest in (('amd64', ''), ('amd64', 'sha256:abc'), ('amd64', AMD64 + '\nextra=value'), ('amd64', 'sha256:' + 'g' * 64), ('other', AMD64)):
            with self.subTest(arch=arch, digest=digest), tempfile.TemporaryDirectory() as directory:
                output = Path(directory) / 'outputs'
                result = subprocess.run(['bash', '-c', step_script('Export digest')], env={**os.environ, 'ARCH': arch, 'DIGEST': digest, 'GITHUB_OUTPUT': str(output)}, capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(output.exists())

    def run_manifest(self, changes: dict[str, str]) -> tuple[int, list[str] | None]:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            recorded = root / 'arguments.json'
            docker = root / 'docker'
            docker.write_text('#!/usr/bin/env python3\nimport json,os,sys\nfrom pathlib import Path\nPath(os.environ["RECORDED"]).write_text(json.dumps(sys.argv[1:]))\nsys.exit(int(os.environ.get("DOCKER_EXIT", "0")))\n')
            docker.chmod(0o755)
            env = {**os.environ, 'PATH': directory + os.pathsep + os.environ['PATH'], 'RECORDED': str(recorded), 'REGISTRY_IMAGE': 'ghcr.io/example/app', 'AMD64_DIGEST': AMD64, 'ARM64_DIGEST': ARM64, 'IMAGE_TAGS': 'ghcr.io/example/app:main\nghcr.io/example/app:sha-test', **changes}
            result = subprocess.run(['bash', '-c', step_script('Create manifest list and tags')], env=env, capture_output=True, text=True)
            return result.returncode, json.loads(recorded.read_text()) if recorded.exists() else None

    def test_manifest_publishes_two_immutable_sources_and_all_tags(self) -> None:
        status, args = self.run_manifest({})
        self.assertEqual(status, 0)
        self.assertEqual(args, ['buildx', 'imagetools', 'create', '-t', 'ghcr.io/example/app:main', '-t', 'ghcr.io/example/app:sha-test', 'ghcr.io/example/app@' + AMD64, 'ghcr.io/example/app@' + ARM64])

    def test_missing_duplicate_or_invalid_inputs_never_call_docker(self) -> None:
        for changes in ({'AMD64_DIGEST': ''}, {'ARM64_DIGEST': ''}, {'ARM64_DIGEST': 'invalid'}, {'ARM64_DIGEST': AMD64}, {'IMAGE_TAGS': ''}):
            with self.subTest(changes=changes):
                status, args = self.run_manifest(changes)
                self.assertNotEqual(status, 0)
                self.assertIsNone(args)

    def test_registry_failure_is_not_suppressed(self) -> None:
        status, args = self.run_manifest({'DOCKER_EXIT': '17'})
        self.assertEqual(status, 17)
        self.assertIsNotNone(args)


if __name__ == '__main__':
    unittest.main()
