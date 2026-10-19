# PWA update recovery delivery

## Implemented

- Updates no longer show the registration-failure message for unrelated failures. Offline, check/download timeout, activation failure and missing registration use their own localized recovery copy.
- One 15-second deadline covers the entire attempt. Listeners and the timer are disposed on completion; late events do not activate or reload. A previously activated controller is recognized on explicit retry, unchanged/null controllers never count as success, and a successful no-update check clears stale notices.
- Missing registration is retried within the same deadline; a failed registration attempt no longer discards an already known registration. Concurrent update calls share one promise.
- Installation and update use one existing HIPPO sheet: same brand, light/dark colors, corners, primary/later/close controls, focus trap and scroll lock. Update copy wraps at narrow widths. The initial mounted transition acquires focus; busy operations focus the dialog when buttons disable. Safe routes, install frequency cap, offline restrictions and Tauri isolation remain intact.

## Verification

- Final `npm --prefix mobile run release:gate`: exit 0, 871/871 tests, both type checks, PWA/Tauri builds and artifact checks, bundle/source-size/behavior budgets. No thresholds changed. Log: `/tmp/pwa-update-release-gate.log`.
- Regression tests execute real update logic and SFC setup, including timeout/retry, already-activated recovery, controller loss, one shared deadline, late registration/download, stale notices, busy guards, initial modal and install/update priority.
- Ego Browser real local service worker: detected new version, explicit activation/reload, forced activation timeout at 15 seconds, restored worker retry/reload, stale notice cleared without an error.
- Keyboard tests: forward/reverse Tab containment, Escape, close focus return to the trigger, scroll unlock, busy focus containment and reduced-motion functional spinner. Offline update disables action with reconnect copy. Synthetic retained native-install event invoked exactly once, kept focus while busy and closed on acceptance.
- 32 layout cases: install/update × Chinese/English × light/dark × 320×568, 390×844, 448×900 and 844×390. Zero page/dialog overflow; update error text is fully visible. Short sheets scroll. Screenshots `/tmp/pwa-update-light-390.png`, `/tmp/pwa-update-dark-en-320.png`, `/tmp/pwa-install-light-390.png` were visually inspected.
- `git diff --check` and task validation passed. Existing new-coin changes were preserved and included in the full test run.

## Release note and limitations

The reported production origin currently has an active worker and all 125 checked precache/worker resources are reachable. Its earlier failure was not reproduced, so the exact historical cause is unconfirmed. The worker response still advertises `Cache-Control: max-age=14400`; release operations should apply the existing `mobile/README.md` no-strong-cache policy to HTML, worker and manifest, including CDN invalidation. Application changes do not alter CDN/server configuration.

No production business writes, cache clearing, deployment. Physical iOS/Android installed apps were not tested. Browser fixtures use a local worker and a synthetic install event, not an OS installation. Ego Lite reported an available update; it was not installed.

Implementation is complete. Business commit `8ef0a37` created; pushed to `origin/main`. `.vite/` remains excluded. The task-owned preview server and task space are closed after verification; pre-existing development services remain running.

## Commit result

Business commit `8ef0a379421e28b41932466be95e5e66145c1ff7` is recorded. Task archive and journal follow before a normal remote push. Latest combined Mobile gate passed 871/871; no business source changed during submission.

Remote verification: normal push to `origin/main` succeeded; `git ls-remote` matched local HEAD `ead6af5e0b6ba20e941f2fc8e05beaed14d06cfb`. This final delivery record is committed and pushed afterward. No deployment.
