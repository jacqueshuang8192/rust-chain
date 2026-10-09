# Delivery

## Changes

- Both PR and publication build jobs now bootstrap the container driver from
  `mirror.gcr.io/moby/buildkit:buildx-stable-1`.
- BuildKit itself uses the same mirror for Docker Hub base-image requests.
- Manifest publication uses the built-in Docker driver, avoiding an unnecessary
  BuildKit image pull and container startup.
- Native build matrix, GHA caches, GHCR credentials/publication and disabled
  CI tests remain unchanged.

## Evidence

- Google cache manifest HEAD returned HTTP 200.
- `docker buildx imagetools inspect` succeeded; index digest:
  `sha256:cec9f139f45e93c5c69c60f8b07cfad9f43f4ef6b6a6cd917527fea5ff2e3dea`.
  Both linux/amd64 and linux/arm64 manifests are present.
- Verified upstream Moby buildx-image workflow publishes the source
  `moby/buildkit:buildx-stable-1` tag; Google cache is a mirror, not a new vendor.
- Source integrity scanner: 16 inputs passed.
- Local source/workflow regression tests: 18/18 passed.
- Ruby YAML parsing and job dependency checks passed.
- Local Docker daemon is unavailable, so no container bootstrap/build was run.
  This is not verification of GitHub Runner connectivity or a successful CI run.
- Mirror availability remains an external dependency; the bootstrap tag must
  exist in the cache. Do not claim the change makes all builds network-independent.

## Commit Scope

Workflow, source-integrity regression test, container delivery spec and this task.
Shared PROGRESS.md retains older uncommitted records and remains local.
The prior PWA commit is not modified; no automatic push or production deployment.
