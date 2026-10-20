# Commit manifest

Work commit: `fix: persist Cargo dependencies across CI image builds`

- `Dockerfile`
- `tests/docker_image_contract.rs`
- `docs/deployment/docker.md`
- `docs/superpowers/PROGRESS.md`
- `.trellis/spec/backend/container-delivery.md`
- `.trellis/tasks/10-20-ci-cargo-download-recovery/check.jsonl`
- `.trellis/tasks/10-20-ci-cargo-download-recovery/commit-plan.md`
- `.trellis/tasks/10-20-ci-cargo-download-recovery/delivery.md`
- `.trellis/tasks/10-20-ci-cargo-download-recovery/implement.jsonl`
- `.trellis/tasks/10-20-ci-cargo-download-recovery/prd.md`
- `.trellis/tasks/10-20-ci-cargo-download-recovery/research/download-path.md`
- `.trellis/tasks/10-20-ci-cargo-download-recovery/task.json`

Existing untracked `.vite/` is unrelated and excluded. Archive only this task
after the work commit, then record the session in its own bookkeeping commit.
No remote push or workflow dispatch is included.
