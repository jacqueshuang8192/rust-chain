# Agent coin application tabs

## Goal
Separate the agent issuance form and own application records using tabs, as requested by the user.

## Requirements
- Reuse Semi Tabs with Chinese labels 新币发行申请 and 我的申请记录; default to the form.
- Keep drafts and visited history state mounted across tab switches; show one accessible panel at a time.
- Preserve existing submission, unknown-result recovery, filtering, pagination and detail behavior.
- Update references to records below the form to refer to the records tab.
- Keep the admin review queue unchanged.

## Acceptance Criteria
- [x] Tabs display their corresponding content and preserve entered form values.
- [x] Submission refreshes records and retains success feedback.
- [x] Existing workflow tests and web quality checks run; browser checks cover desktop and narrow layouts.
- [x] Progress and UI contract updated.

## Technical Approach
Use Tabs.TabPane with keepDOM, lazyRender and disabled panel motion. Lazy history mounting avoids measuring a hidden table; keeping mounted panels preserves drafts, filters and column widths. No API, routing, backend or dependency changes.

## Plan
1. Inspect page and existing tab conventions.
2. Apply tab composition and update affected workflow checks.
3. Validate, document outcomes and update progress.

## Scope
Only agent coin application presentation and associated documentation/verification. No admin review redesign, automatic tab switching, submission, push or deployment.

## Verification outcome
- Web: 1083/1083 tests, 15/15 production-policy tests, 23/23 coverage tests and thresholds, lint, typecheck and production build passed.
- Browser: both tabs at 1728/1280/390/320px have one visible panel and no document overflow; draft persistence, keyboard selection and successful submission refreshing visited history passed with in-memory API fixtures. Screenshots inspected.
- Existing bundle budget remains failing at 2677241 / 2600000 JS bytes (previous recorded baseline 2676877). No threshold change.
- Changes remain uncommitted; no push or deployment.
