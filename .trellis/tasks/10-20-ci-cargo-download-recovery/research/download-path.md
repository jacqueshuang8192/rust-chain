# Cargo download failure in GitHub image builds

The supplied amd64 log fails in Cargo index/crate downloads, before compilation.
The underlying network cause (runner egress, DNS, CDN outage or other transport
failure) cannot be proven from this excerpt. The local gh account cannot read
the repository Actions API (404), so no remote rerun is currently available.

The existing workflow already exports architecture-scoped GHA max layer caches,
but registry/git mutable cache mounts are not included by default. Splitting
fetch into an ordinary layer keyed on the Rust base and Cargo manifests lets
later source-only builds reuse downloaded dependencies across ephemeral runners.
The dependency directories must not be hidden again by mounts during compilation.

Options considered:
- Only increasing retries: buffers transient failure but still downloads again
  after source edits on new runners; incomplete for the discovered cache gap.
- Registry mirror or host networking: no evidence to select a reliable mirror or
  blame Docker bridge routing; avoid unrelated provenance/network-policy changes.
- Cached locked fetch plus offline compile: uses existing cache exporters and
  native runner contract; selected, with finite retry and hard fetch timeout.

The current explicit lib/bin target paths allow cargo fetch before copying src.
A temporary manifest-only offline probe reached dependency resolution and failed
only on an uncached Linux crate; no dummy executable sources are required.

Primary references consulted:
- https://doc.rust-lang.org/cargo/commands/cargo-fetch.html
- https://doc.rust-lang.org/cargo/reference/config.html
- https://docs.docker.com/build/ci/github-actions/cache/

Limits: Cargo offline mode forbids Cargo registry updates/downloads, but arbitrary
crate build scripts can still use the network; do not describe the entire RUN as
network-isolated without separately validating every build script.
