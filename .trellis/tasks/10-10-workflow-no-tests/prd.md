# GitHub Workflow Without Tests

- Remove automated test execution from the Docker GitHub workflow, including
  frontend, Rust, scanner unit tests and real-dependency smoke tests.
- Retain source integrity scanning, formatting, lint, type checks, frontend
  builds and container publication dependencies on static checks.
- Keep the local full release gate unchanged.
- Validate workflow structure and source-integrity regression checks locally.
- Do not include prior mobile or market-label changes in this task's commit.
