# Registration Legal Documents

## 1. Scope

Mobile and PC registration require a published Terms of Service / Privacy
Policy pair. Enabled countries use a global pair until their own pair is
published. This is a publication and evidence workflow, not approved legal text.

## 2. Signatures

- `GET /api/v1/auth/register/legal?country_code=CN&locale=zh-CN` returns one
  `RegistrationLegalBundle`: publication ID, actual scope/locale/edition, both
  immutable document IDs, titles, plain text and effective dates. Anonymous.
- `POST /api/v1/auth/register` adds `legal_publication_id`, `legal_locale`,
  `legal_acknowledged=true`.
- `POST /api/v1/auth/register/email-code` requires the same country and
  publication fields before code generation or delivery. Both clients show the
  notice before submitting this email request.
- Admin `GET/POST /admin/api/v1/legal-documents`, `GET` by publication ID and
  user receipt ID require `system.legal.read/write` permissions.
- Tables: `registration_legal_documents`, `registration_legal_publications`,
  `registration_legal_current`, `user_registration_legal_receipts`.

## 3. Contracts

Published documents and pairs are immutable. Admin edits are local drafts
until both documents publish in one transaction, with `expected_edition` and
an audit reason. Global scope is `GLOBAL`; country scopes must exist in the
country directory. Resolution prefers exact country/locale, then global
translation, then country default and global English. Never combine documents
from different scopes, languages or editions. Return the actual language.

Registration locks its enabled country and then the current publication
pointer. Missing acknowledgement or ID fails before email code verification.
The email-code sender checks the same current pair and country before checking
SMTP settings, storing a code hash or sending mail.
Publishing a country override takes the same country lock before creating or
updating its publication pointer, including the first publication.
Changed publication returns 409 `LEGAL_DOCUMENTS_STALE`. User insertion and
one receipt with country, scope/locale, publication and document IDs commit
atomically. A failed receipt rolls back account and code consumption. Existing
users have no retroactive receipt; never fabricate one. Public reads expose
only published copy and never user receipts.

Render document bodies as escaped plain text with line breaks. The affirmative
action accepts the terms and acknowledges the privacy notice; marketing consent
is separate. UI requires viewing both tabs and resets review on country, locale
or publication changes. The server stores the asserted action, not a claim to
measure whether every word was read. Immediately published effective dates
cannot be in the future.

Operator-authored content and country-specific legal review are prerequisites
for actual compliance. Lawful bases, data practices, rights, age gates and
jurisdictional notices cannot be inferred from a screenshot or country code.
Migration seeds no legal text. Publish reviewed global copy before expecting
registration on a new backend. Older cached PWA clients will be rejected until
updated.

## 4. Validation And Errors

| Condition | Result |
| --- | --- |
| Scope unknown / invalid locale | 400 |
| Missing published pair | 409; no account/code consumption |
| Missing affirmative action / ID | 400; no account/code consumption |
| Stale or wrong-country publication | 409 `LEGAL_DOCUMENTS_STALE` |
| Wrong Admin role | 403 |
| Edition conflict | 409, no partial publish/audit |
| Body outside 80 chars to 128 KiB / future effective date | 400 |

## 5. Cases

Good: global English copy serves an enabled country; a country translation
overrides only when both documents are published in that language. Base: an
older registration ID fails while its email code remains pending. Bad: `#`
links, only a client checkbox, or one old and one new document in one bundle.

## 6. Verification

Real isolated MySQL: global/country and language selection, Admin permissions,
edition conflict, stale/missing acknowledgement, pending email code, immutable
old content, receipt and audit. OpenAPI: all routes and registration fields.
Mobile release gate, PC build, Web checks and phone viewport dialog review.

## 7. Wrong Vs Correct

Wrong: publish hard-coded generic copy, trust `accepted=true`, overwrite text
after users register. Correct: publish an immutable pair, verify its current
ID inside account creation, then record the exact pair in that transaction.
