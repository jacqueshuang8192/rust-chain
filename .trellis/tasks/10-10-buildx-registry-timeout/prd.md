# Buildx Bootstrap Registry Timeout

## Scope

The reported job fails downloading moby/buildkit:buildx-stable-1 from Docker Hub
before the project build. Use the verified Google Docker Hub cache for the
container-driver image in both build jobs and configure the same registry
mirror inside BuildKit for base images. Use the built-in docker driver for the
manifest-only job, which needs no BuildKit container.

Keep CI test execution disabled, native amd64/arm64 builds, GHA cache export,
GHCR publication and existing permissions unchanged.

## Validation

- Inspect mirrored image availability and supported platforms.
- Check workflow YAML/job dependencies and local regression tests.
- Report local vs GitHub Runner network verification boundaries honestly.
- No unrelated PWA or market-label changes, no automatic remote push.
