# Remove Docker artifact quota dependency

## Goal
Fix reported CreateArtifact quota failure without hiding build/publish failures.

## Requirements
- Preserve native amd64/arm64 matrix, integrity/quality gates, GHA build cache, push-by-digest and final tag policy.
- Replace uploaded digest files with uniquely named matrix job outputs. Validate sha256 identities before export and publication; require both outputs.
- Disable Docker build record artifact upload for both PR and publishing builds.
- Do not delete remote artifacts, change billing, weaken gates, or deploy.

## Acceptance
- [x] No artifact upload/download dependency remains in image workflow.
- [x] Matrix outputs have distinct keys, missing/malformed digests fail closed; final command includes both immutable digests and all metadata tags.
- [x] Workflow contracts, shell execution tests and syntax checks pass; limitations documented.

## Research
GitHub official matrix job outputs example permits merging distinct names; same names race. Docker build-push-action v7 defaults DOCKER_BUILD_RECORD_UPLOAD=true. See research/sources.md.

## Scope
Workflow, focused tests, container delivery spec, progress and task records. GitHub API run listing returned 404; no remote CI success claimed.
