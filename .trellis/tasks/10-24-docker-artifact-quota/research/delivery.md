# Delivery — 2026-10-24

## Change

Removed artifact transport for architecture digests; replaced with unique matrix job outputs using GitHub's documented pattern. Disabled Docker build record upload. Kept digest-based GHCR publishing, both architecture runners, tag policy, cache and all existing gates. Manifest fails closed for missing/invalid/duplicate digests and empty tags; registry errors propagate.

## Verification

- `python3 -m unittest tests.test_docker_workflow_outputs tests.test_source_integrity_gate`: 24 passed.
- `python3 scripts/source_integrity_gate.py`: passed, 16 build inputs.
- `cargo test --test docker_image_contract`: 10 passed.
- YAML parsed (4 jobs); extracted workflow scripts passed shellcheck.
- actionlint v1.7.12 passed for `.github/workflows/docker-image.yml`. Go module launch produced no progress and was stopped; used official release binary verified against official SHA256 checksums instead.
- `git diff --check` and task validation passed.

## Limits / rollout

GitHub run listing returned 404 with available CLI credentials. No remote artifact deletion, billing changes, push, rerun, GHCR publication or deployment was performed. Local shell tests use a Docker recorder; matrix aggregation is based on the official GitHub contract, not a live runner test.

Push this revision and run the corrected workflow to validate remote publication. Retrying only the previous failed revision retains its old artifact steps. This removes this workflow's upload dependency; it does not reclaim account storage. Existing reported quota recalculation delay remains an account concern.
