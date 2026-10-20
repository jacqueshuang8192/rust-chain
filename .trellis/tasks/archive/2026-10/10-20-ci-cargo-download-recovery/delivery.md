# CI Cargo download recovery

## Outcome

The reported amd64 GitHub Actions log failed during crates.io index/crate
downloads. It did not show a Rust source compilation error. The underlying
network outage cannot be identified from the excerpt, and the locally configured
GitHub API identity cannot read this repository's Actions runs (404).

Docker now fetches locked dependencies for its native Rust host in a dedicated
`rust-dependencies` stage before copying application sources. That stage stores
registry/git content in ordinary layers exported by the existing per-architecture
GHA max cache. It uses Cargo's five network retries, 120-second HTTP timeout and
a 15-minute outer deadline with a ten-second kill grace period. The compiler
inherits those files and runs locked/offline, retaining only the target mount.
Persistent download failures and compiler errors still fail the build.

No workflow YAML, lockfile, dependency version, runtime image contract or business
code changed. The obsolete Docker test expectation for a service smoke job was
aligned with the existing 2026-10-10 user decision to run static checks/builds in
CI. The workflow itself still waits for that quality gate before either build.

## Verification

- `cargo test --test docker_image_contract --test backend_architecture --test backend_documentation`: 10 + 11 + 1 passed. Docker contract tests also pass after the final lint-only correction.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --all -- --check`: passed.
- `python3 scripts/source_integrity_gate.py`: passed, 16 build source inputs.
- `python3 -B -m unittest tests.test_source_integrity_gate`: 18/18 passed.
- Parsed workflow YAML and verified triggers, native architecture runners, quality
  dependencies, local context, architecture-specific GHA max caches, PR read-only
  permissions and manifest dependency.
- `bash -n docker/supervise.sh` and both documented `docker compose ... config --quiet`
  commands: passed.
- `npm --prefix web run build`: passed with the existing chunk-size warning.
- Real manifest-only `cargo fetch --locked --target` followed by offline fetch
  passed for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`; Cargo.lock
  was byte-identical afterwards. No dummy application sources were needed.
- `docker build --platform linux/arm64 --target rust-dependencies`: passed.
  A fresh container from that image with `--network none` and no cache mounts
  successfully ran native `cargo fetch --locked --offline`, proving dependencies
  are inside the image. The next application build reported this fetch as CACHED.
- `docker build --platform linux/arm64 --target builder`: passed; the real locked
  offline release compile finished in 7m35s. A fresh container confirmed both
  installed executable files exist outside the target cache: exchange-api
  50,884,320 bytes and exchange-migrate 4,988,128 bytes.
- `git diff --check` and task context validation: passed.

Logs: `/tmp/ci-cargo-contract-tests.log`, `/tmp/ci-cargo-contract-final.log`,
`/tmp/ci-cargo-clippy-final.log`, `/tmp/ci-cargo-source-tests.log`,
`/tmp/ci-cargo-web-build.log`, `/tmp/ci-cargo-native-fetch.log`,
`/tmp/ci-cargo-deps-arm64.log`, `/tmp/ci-cargo-builder-arm64.log`.

## Limits and publication

The local amd64 dependency-image attempt was stopped after the Rust base-image
blob download stalled for several minutes, before the new Cargo stage ran.
Linux amd64 Cargo dependency resolution/downloads were verified separately, but
the complete amd64 image and GitHub Actions rerun remain unverified. Cargo
offline mode does not block arbitrary third-party build-script networking.

This is download recovery/cache hardening, not a guarantee against a persistent
runner/CDN outage. No push, workflow dispatch, GHCR publication or deployment
was performed. Pushing the eventual commit will let the existing workflow run
the updated Dockerfile. Existing unrelated `.vite/` is excluded from commits.
The two task-specific Docker image tags are removed after verification; existing
images, services and shared build caches are preserved. The integrated runtime
image was not rebuilt or started; runtime and frontend source did not change.
