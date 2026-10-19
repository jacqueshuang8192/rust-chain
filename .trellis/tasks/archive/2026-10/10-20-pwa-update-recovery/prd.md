# PWA update recovery and install-style dialog

## Goal

Fix misleading update failures and recover the PWA update flow. Present updates using the existing installation bottom sheet, as requested, while preserving the pending new-coin changes.

## Requirements

- Distinguish registration, update-check/download, activation, and offline failures with localized actionable copy.
- Recover when another tab or a previous timed-out attempt has already activated the worker; never reload on controller loss or an unchanged controller.
- Bound an update attempt by one deadline, clean up listeners/timers, prevent concurrent attempts, and allow retry without clearing storage or unregistering workers.
- Share the installation sheet's brand, theme, buttons, motion and modal accessibility. Updates retain highest priority and safe-route gating. Busy state prevents duplicate actions and dismissal.
- Preserve install eligibility/frequency cap, offline notices, shell-only caching, Tauri isolation and all existing business behavior.

## Acceptance Criteria

- [x] Lifecycle tests cover success, stale notice, already-activated recovery, rejected checks/messages, redundant workers, controller loss, one total deadline, late events and retry.
- [x] Executable UI tests cover shared modal priority, initial display, busy guards and localized error selection.
- [x] Browser checks cover update/install, retry/offline, keyboard focus, light/dark, Chinese/English and narrow/short screens.
- [x] Mobile release gate passes without increasing budgets; spec and progress records updated.

## Plan

1. Inspect existing registration/update logic and release assets; record evidence and reproduction limits.
2. Repair update state handling and share the installation sheet.
3. Run focused regressions, full release gate and browser checks; record delivery.

## Known Findings

- `PwaStatus.vue` maps every update failure to `pwa.registrationFailed`.
- `runPwaUpdate` currently restarts its timeout at each phase and accepts any controllerchange event; retry cannot recover a worker that activated after a timeout.
- `useModalDialog` needs an open transition after mounting. Use one shared modal lifecycle to avoid competing scroll locks/focus during install/update changes.
- Public HTTP inspection of the reported origin currently receives 403; this alone does not identify the user's service-worker failure.
- Existing installation geometry and palette are the requested design reference; no unrelated redesign or new dependency is needed.

## Scope

Mobile PWA lifecycle, shared prompt, locale copy, regression tests and their docs only. No production deployment, storage clearing, business writes, backend changes or remote push. Preserve all previously verified new-coin changes.
