# Delivery

Implemented immutable registration Terms and Privacy publication for global or
country/language scopes, current-pair resolution, Admin authoring/history and
user receipt inspection. Mobile and PC display both full plain-text documents
in a reading dialog and submit the current publication ID only after review.
Registration verifies that ID inside the existing account transaction before
email-code consumption and writes one immutable user receipt there.
Registration email-code sending validates the same country/publication before
generating or sending a code; the clients show the notice before sending the
email request.

## Verification

- Real isolated MySQL: new Admin publication/read permission, global fallback,
  country override, localized global copy, stale version, missing acknowledgement,
  unsent email code on invalid acknowledgement, unchanged pending email code,
  immutable prior edition, country-row publication serialization, account and receipt:
  focused integration passed on a fresh test schema.
- Existing country and invitation registration routes passed 2/2 in a separate
  clean test schema after adding the email-code notice fields.
- Rust architecture 11, documentation 1, OpenAPI 15 passed; strict all-target
  Clippy, formatting and source integrity passed.
- Mobile full release gate: 844/844 tests, PWA/Tauri Web builds, artifact and
  bundle checks passed. PC type check and production build passed.
- Web full tests: 112 files, 1064/1064. Type check, lint, build, production
  policy 15 and coverage suite 23 passed. JS raw bundle budget remains over:
  2,657,217 / 2,600,000 bytes. This limit was already exceeded before this
  task and was not relaxed.
- Ego Browser: real Vue rendering on Mobile and PC at 320x720, both tabs,
  disabled-to-enabled acknowledgement, no horizontal overflow, mobile country
  switch clearing prior consent. Read-only local API fixture used; no signup POST.
- Full shared-database `user_routes` suite has one unrelated pre-existing invite
  fixture failure: it inserts two codes for the same agent against a unique
  `(owner_type, owner_id)` constraint. The new legal integration passes in
  isolation and in that suite; the full suite is not green.

## Rollout

Migration `0149_registration_legal_documents.sql` contains no legal copy.
After deploying backend and both clients, an authorized editor must publish
reviewed global Terms and Privacy before registrations can succeed. Older
cached clients lack the version fields and will be refused by the new server.
For country-specific law and translations, the operator must provide factual
content and legal review. The mechanism cannot certify substantive compliance
from a screenshot. No production database was changed; no push or deployment.

## Follow-up: Rich Text And Publish Usability

- Removed the undisclosed 80-character prerequisite. The Admin page now lists
  missing publication fields; the existing Quill editor authors allowed text
  blocks, and its fixed container height keeps the publish action clickable.
- Migration 0150 preserves old plain-text rows and marks new structured
  editions `rich_json`. The backend rejects unsupported nodes and size/empty
  violations; Admin history, Mobile and PC render safe text nodes.
- The isolated MySQL publication/registration test, strict Rust Clippy,
  OpenAPI/architecture tests, Mobile release gate, PC build, Web 1064 tests,
  type/lint/build/policy/coverage passed. The existing Web JS budget remains
  over its 2,600,000-byte limit (2,658,813 bytes); it was not relaxed.
- In a local browser with an in-memory API fixture, 390px editing enabled the
  button for short copy, the modal submitted both rich documents, and the
  editor no longer intercepted the action. No production write occurred.

## Follow-up: Mobile Reading Sheet

- Replaced the desktop-style reading dialog with a 448px-max bottom sheet using
  the existing registration theme boundary, 24px top corners, circular Close,
  segmented document tabs and mint acknowledgement. Header and actions remain
  visible while the long document scrolls, with safe-area and reduced-motion CSS.
- Fixed the conditional component's modal lifecycle so the existing watcher
  receives an opening transition; dismissal now restores body overflow and
  trigger focus. Tabs support arrows/Home/End and reset document scroll.
- The full Mobile release gate passed: 844/844 tests, both type checks, PWA
  and Tauri web builds, artifact assertions, unchanged bundle/source budgets
  and critical-test governance. Initial bundle failures were resolved by
  simplifying this component; no limits were raised.
- Browser verification used an in-memory read-only API fixture, not production:
  light/dark 320x568, 390x844, 448x900, 844x390 and 1280x800 had no horizontal
  overflow or footer overlap. Verified review gating, tab keyboard navigation,
  focus loop/return, Escape/cancel/backdrop, locale reset, plain/rich text,
  English long-word wrapping and reduced motion. After waiting for emulated
  layout updates, simulated 20px side/34px bottom insets retained visible actions.
- Screenshots were inspected. Physical devices and installed PWA safe areas
  were not tested. The regular dev server is on port 5186; test overrides were
  browser-memory-only and the test TaskSpace was closed.

## Follow-up: White Theme And Date Removal

- Refined the light reader with a white surface, underline tabs, inline
  language label, 20px document title, 15px body, smaller section headings,
  low-emphasis Cancel and dark-green/white acknowledgement. The dark palette
  remains unchanged. Removed effective/last-updated date rows from both themes
  without changing API metadata, publication identity or consent behavior.
- Styles use the component-owned `legal-*` namespace. Removed duplicate
  box-sizing/wrapping declarations and scoped-selector overhead so the
  existing bundle budgets pass without raising limits. Document-title CSS
  targets its direct child and does not leak into rich-text headings.
- Final Mobile release gate passed: 844/844 tests, both type checks, PWA/Tauri
  builds, artifact assertions and unchanged bundle/source/test budgets.
  Browser checks covered light/dark 320x568, 390x844, 448x900, 844x390 and
  1280x800, English/plain/rich text, long words and reading/focus behavior.
  A full reload (not hash navigation) verified the final stylesheet and a
  second ten-layout pass, hidden dates, acknowledgement and Escape/focus return.
- Reviewed final 390px screenshot; the browser used in-memory read-only
  fixtures and its TaskSpace was closed. No production data, push or deployment;
  physical-device validation remains outstanding.
