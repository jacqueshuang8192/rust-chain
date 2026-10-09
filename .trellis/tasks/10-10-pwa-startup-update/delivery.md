# Delivery

## Implementation

- Controlled online PWA startup now awaits registration/update/activation before
  mounting Vue, with one 4-second deadline across the whole sequence.
- Ready workers receive SKIP_WAITING and reload only on a changed non-null
  controller. Observers/timer are removed on every completion path.
- Late completion never automatically activates/reloads the mounted application.
  First visits/offline visits remain non-blocking; post-mount prompts remain.
- No service-worker precache, API caching, Tauri policy, credentials or order
  behavior changed. No cache clearing or user logout.
- README covers no-store worker/HTML headers, CDN invalidation, atomic rollout,
  retention of old hashed assets and correct PWA vs Tauri output.

## Verification

- Focused PWA tests: 22/22 passed.
- Full `npm --prefix mobile run release:gate`: 817/817 tests, zero skips,
  app/test types, PWA/Tauri builds, artifact and bundle/governance checks passed.
- Ego TaskSpace6: real Chromium + generated Workbox prompt worker + production
  startup helper, served on localhost under `/mobile/`.
  v1 controlled -> deploy v2 -> ordinary reload -> v2, no waiting worker.
  Deploy v3 + background update -> v2 remained mounted with waiting worker.
  Revisit -> v3. Offline reload -> cached v3.
- Fixture used only local static content; no production APIs or financial state.
  TaskSpace6 closed, temporary fixture server stopped.
- Task context validation and `git diff --check` passed.

## Boundaries

- Browser fixture validates worker lifecycle, not deployed CDN headers or the
  full authenticated application. No production URL was supplied.
- Existing legacy cached code must accept an update once (or close all clients
  after download) before it can execute the new startup policy.
- Android/iOS native packages were not rebuilt; browser/Tauri isolation was
  checked through unit tests and the Tauri frontend build.
- Isolated commit includes main.ts, pwa/index.ts, pwa/startup.ts, startup tests,
  mobile README, shell spec and this task. PROGRESS.md has previous uncommitted
  task entries and remains local. No push or deployment in this task.
