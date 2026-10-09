# Delivery

- GitHub Docker workflow now runs static checks and frontend builds, without
  Rust/frontend/scanner tests or real-dependency smoke jobs.
- Rust Clippy targets production libraries and binaries, not test targets.
- Container publishing still depends on the quality job. Local full release
  scripts and real-dependency test sources remain unchanged.
- Validation: source integrity scanner passed (16 inputs), scanner/workflow
  unit tests 17/17 passed, Ruby YAML parsing and job dependency checks passed.
- actionlint is not installed. No remote Actions execution or container build
  was performed; CI success will not prove database/service compatibility.
- Commit scope: workflow, source-integrity regression test, governance spec,
  and this task directory. Shared PROGRESS.md retains prior uncommitted entries
  and is deliberately excluded from the isolated workflow commit.
- Previous mobile and market-label changes are untouched; no push requested.
