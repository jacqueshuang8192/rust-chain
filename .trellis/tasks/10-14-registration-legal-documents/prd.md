# Registration Legal Documents

## Goal

Present actual Terms of Service and Privacy Policy during user registration on Mobile and PC, allow administrators to edit and publish country/language variants, and record the exact accepted versions atomically with account creation.

## Known Requirements

- Cover all countries currently open for registration, using the existing country configuration as the scope.
- Use the supplied screenshot as a reading-dialog interaction reference, not as legal copy or branding.
- Both documents must be available before registration; the Admin editor owns the reviewed legal text and publication.
- Terms acceptance and privacy notice acknowledgement must not be bundled with marketing consent.
- Terms and privacy revisions stay immutable after publishing; admin changes create new versions.
- The server validates country, selected document versions and affirmative action in the registration transaction, including races with publication.
- The registration email-code endpoint also checks the current notice before generating or sending a code; both clients show the notice before submitting an email request.
- Existing non-user admin/agent auth must remain unaffected.
- No fabricated legal language or claim of automatic compliance with every country's law.

## Decision

- User selected one global version for all currently open registration countries. Country-specific published bundles can override it later.
- A Terms/Privacy pair is published atomically. No legal boilerplate is auto-published; administrators must enter and publish reviewed global copy before the new registration gate can be used.
- Public document resolution prefers a matching country/locale pair, then a global translation, then a global English pair. The response exposes the actual language shown.

## Acceptance

- Public country/language document reads expose only published content, with explicit fallback locale.
- Mobile and PC show both full documents in an accessible reading flow; country/locale changes invalidate prior acknowledgement.
- Server rejects omitted, stale, mismatched and unpublished revisions without creating a user or spending an email code.
- One immutable consent receipt is written in the same transaction as user creation; admin can inspect revisions and receipts under permissions.
- Saving drafts never silently publishes; publishing both documents is an explicit admin action with audit attribution.
- Editing is local to the Admin page until publication; persistent server-side drafts are outside this MVP.
- Regression, type, lint and build gates run for changed packages.

## Scope Limits

- Product mechanism and review workflow only. Legal counsel must supply the operator identity, data practices and jurisdiction-specific text.
- No IP-based jurisdiction guess and no claims that a checkbox alone establishes a GDPR processing lawful basis.

## Codebase Notes

- Public user register: `src/modules/auth/application.rs`, `routes.rs`; registration already locks configured country and consumes email code in MySQL transaction.
- Mobile register: `mobile/src/views/RegisterView.vue`; PC register: `pc/src/views/auth/Register.vue`; both currently lack functional document links.
- Admin uses permission-guarded lazy action pages and existing country configuration.

## Verified Delivery

- The global pair and optional country/language overrides use immutable revisions and one atomic current publication pointer.
- Both Vue registration clients show full plain-text terms and privacy; country/language changes clear acknowledgement.
- Server-side registration validates the current pair before consuming email code and stores one immutable receipt with the user.
- Exact API, RBAC, release gates and 320px browser interaction were checked. Final test counts and limitations are in `delivery.md`.
