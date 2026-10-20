# Repair Cargo dependency downloads in GitHub image builds

## Goal
Resolve the reported GitHub Actions image build failure while downloading Rust
dependencies from static.crates.io, preserving the existing release artifacts.

## Known facts
- User confirms the failure is in GitHub Actions, not a server-side Docker build.
- The provided log shows Cargo connection failures during the release build.
- Dockerfile currently fetches dependencies inside cargo build and uses BuildKit
  cache mounts for the Cargo registry, Git dependencies and target directory.
- The repository has one image workflow with existing native-platform delivery
  constraints. Inspect it and available run logs before choosing the repair.
- User supplied the amd64 log: the index low-speed timeout is followed by many
  static.crates.io connection failures, before Rust compilation starts.
- Local GitHub API access returns 404; do not claim to have inspected or rerun CI.
- Both build jobs already export normal layers via architecture-scoped GHA max
  caches. Mutable Cargo cache mounts are not exported by that configuration.

## Decision and plan
1. Fetch locked dependencies for the native Rust host in a source-independent
   normal image layer, before copying migrations and Rust sources. The manifest
   declares explicit targets, so dependency resolution requires no dummy code.
2. Use Cargo's built-in retry policy (5 retries), 120-second HTTP timeout and a
   15-minute outer fetch deadline. Do not retry compilation or switch registries.
3. Compile with locked, offline Cargo using those persisted dependencies; keep
   only the target directory as a mutable cache mount.
4. Test cache/ordering contracts and the real manifest fetch/offline behavior;
   run available build checks and record any unavailable Docker/CI verification.

The dependency layer has its own named stage so it can be built and verified in
an isolated container without compiling application code. The workflow already
exports this normal stage and needs no YAML edit. Align the obsolete Docker
contract assertion with the recorded 2026-10-10 static-only CI decision.

This reduces repeated cold downloads and tolerates transient failures; it does
not claim to cure a sustained GitHub-runner/CDN outage. The user request already
authorizes this build repair; no unresolved product decision is needed.

## Requirements
- Limit changes to CI/dependency download reliability and related documentation.
- Preserve Cargo.lock, locked builds, architecture targets and publication rules.
- Do not hide compilation failures behind download recovery or weaken checks.
- Validate executable build configuration using the project's existing guards.

## Acceptance
- [x] Identify what is proven by the failing log and what remains uncertain.
- [x] Apply and verify the smallest justified CI/download repair.
- [x] Record validation and limits, including any unavailable remote run access.

## Out of scope
Business logic, dependency upgrades, server deployment and unrelated UI changes.
No remote push or workflow dispatch is performed by this task.
