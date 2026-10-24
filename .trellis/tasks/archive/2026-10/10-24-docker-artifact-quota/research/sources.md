# Official source verification

- GitHub workflow syntax: https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idoutputs
- Corresponding primary source fetched read-only: https://raw.githubusercontent.com/github/docs/main/data/reusables/actions/jobs/section-defining-outputs-for-jobs.md . Matrix job outputs merge distinct output names; same names may be overwritten by nondeterministic completion order. Follow this documented example using digest_amd64/digest_arm64.
- Docker action: https://github.com/docker/build-push-action/tree/v7 . README confirms DOCKER_BUILD_RECORD_UPLOAD defaults to true and false disables GitHub artifact upload.
- web.run returned no readable content; primary source retrieved via urllib to /tmp/gha-job-outputs.md and /tmp/gha-build-push.md.
- Repo currently uploads empty digest files with retention-days: 1. Shortening retention further is not the fix; it still requires quota. User error states quota accounting recalculates every 6–12 hours.
- Chosen fix: native job outputs and disable optional build record upload. Avoid new registry staging tags or destructive remote cleanup. Existing Actions cache stays unchanged.
