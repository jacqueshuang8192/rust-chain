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
