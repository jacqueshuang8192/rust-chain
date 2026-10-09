# PWA Startup Update After Deployment

## Root Cause

The generated worker serves the precached shell. Updates wait for explicit
acceptance, but the acceptance UI only appears on safe routes. Refreshing a
controlled page does not itself activate the waiting worker.

## Scope

- Before mounting an already-controlled PWA, check registration and updates.
- Activate a ready update and reload only during a bounded startup window.
- Release startup on timeout, registration/check failure, or no update; late
  updates must use the existing user-controlled prompt, not reload live trading.
- First visits, offline fallback, Tauri isolation and API cache exclusions stay.
- Add deployment guidance for non-cached HTML/worker and atomic asset rollout.
- Document initial migration: already-installed old code needs one explicit
  update (or all tabs closed) before the new startup policy can execute.

## Validation

Deterministic lifecycle tests (waiting/installing/no update/offline/errors/late
completion), full mobile release gate and a two-version local worker check.
No production deployment or financial mutations.
