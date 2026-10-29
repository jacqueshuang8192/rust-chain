# Complete the PWA/H5 login subtitle

## Goal

Apply the user correction: title `登录或注册`, subtitle `欢迎！请登录以开始使用`.

## Scope and plan

Update only `auth.pencilLoginDescription` in the existing Chinese mobile locale. Preserve the title, keys, other locales and authentication behavior. Use the existing release gate plus direct locale-value verification; no new test or spec contract is needed for this one-character copy correction. Record progress and commit/archive locally; no push or deployment.

## Acceptance criteria

- [x] Production locale contains the exact two lines requested.
- [x] Mobile release gate and diff validation pass.
- [x] Progress and task records capture the correction.

## Verification

- Imported the production locale through Node TypeScript stripping and asserted the exact title and completed subtitle.
- `npm --prefix mobile run release:gate` passed: 923/923 existing tests, production/test type checks, PWA/Tauri web builds, generated artifact assertions, both bundle budgets and governance checks.
- `git diff --check` and task context validation passed. Browser checks were not repeated for this one-character correction; the preceding title edit was inspected at 320px and 390px.
- No new spec or test is needed: the existing localization contract is unchanged. No push, deployment or production access.
