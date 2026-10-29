# PWA/H5 login copy update

## Goal

Replace the two Simplified Chinese login heading lines with the exact text provided by the user.

## Requirements

- `auth.welcomeBack`: `登录或注册`.
- `auth.pencilLoginDescription`: `欢迎！请登录以开始使` (preserve the supplied wording, including its ending).
- Change only the production mobile locale values consumed by `LoginView.vue`; preserve keys, other locales, login/registration flows and historical Pencil files.

## Acceptance criteria

- [x] The production Chinese locale has both exact requested values.
- [x] Login page displays both lines at a narrow viewport without overflow.
- [x] Existing mobile release gate passes and PROGRESS records the result.

## Plan

1. Update the two existing locale values.
2. Run the required mobile release gate and inspect the local login page.
3. Record scope and verification, then commit and archive locally. No deployment or push is requested for this separate copy change.

## Scope

Only mobile/src/i18n/messages/zh-CN.ts, this task and the task-specific progress/session records. No new tests or abstractions for a reversible copy edit; use existing checks.

## Verification

- `npm --prefix mobile run release:gate`: passed, including production/test type checks, 923/923 tests, PWA and Tauri web builds, generated artifact checks, bundle/source budgets and test-quality checks.
- Ego TaskSpace 11, local login route at 320x844 and 390x844: exact title/subtitle present, document scrollWidth equals viewport width, screenshots inspected. The local backend proxy pointed to a closed loopback port; only the existing email-only config fallback rendered. No login request or production API access.
- `git diff --check`: passed. Browser task and Vite server stopped.
- Existing localization conventions already cover this two-value edit; no new spec contract or test was necessary. English/Pencil/PC copy and all auth behavior are unchanged.
