# Agent downline new coin records

## Goal
Expose read-only new coin subscription and post-listing purchase records for users in the authenticated agent subtree.

## Requirements
- Reuse team-order pages and scope: owned users plus descendant-agent users; never parent, sibling, unrelated or unassigned users.
- Add separate Chinese navigation entries 新币申购 and 新币认购. Recognition follows existing purchase orders (post-listing purchases).
- Support existing team/user/email/order/status/time/paging filters plus project ID; new coin symbol filters use project symbol.
- Show exact decimal snapshots, subscription allocation/freeze/payment/refund and nullable historical values. Never expose idempotency keys or financial write controls.
- Preserve stale response protection, details, empty/error handling and narrow-screen table containment.

## Acceptance Criteria
- [x] Both kinds return scoped rows and matching totals; invalid filters rejected.
- [x] Real database verifies ancestor/child/sibling isolation, filters, pagination and no writes.
- [x] API/component tests validate exact amounts, nulls, navigation and read-only details.
- [x] Typecheck/lint/build and backend route/OpenAPI/architecture checks pass; browser checked.

## Out of Scope
Settlement, allocation, refunds, project issuance, production writes, deployment and unrelated coin-application Tabs changes.

## Technical Approach
Extend existing /agent/api/v1/orders/:kind with new-coin-subscriptions and new-coin-purchases; retain static SQL projections and shared rows/count subtree predicate. No schema changes.

## Decisions
Use existing full subtree reporting semantics; user request authorizes visibility only. Two existing business tables determine the meaning of subscription and purchase, with purchase page explicitly labeled post-listing in context.

## Validation Results
- Real isolated MySQL 9.3: team route scope/filter/paging/read-only test, invalid request/auth test and OpenAPI test all passed (3/3); all six order kinds were exercised. Temporary database removed.
- Architecture 11/11, OpenAPI 16/16, documentation 1/1, all-target check, strict Clippy and fmt passed.
- Web 113 files / 1098 tests, typecheck/lint/build, policy15 and coverage23 passed.
- Existing JS total budget remains red: 2,678,862 > 2,600,000 bytes (previous support baseline 2,675,427). Threshold unchanged.
- Ego space10 local in-memory fixture: both pages at1728/1280/390/320 have no document horizontal overflow; historical null detail, project empty/reset, failed fetch/recovery, navigation and 200->230px real column resize passed. 4 list screenshots and1 narrow drawer screenshot inspected; no API writes. Space closed.
- No production deployment, real user trading actions or device testing. Earlier uncommitted coin-application Tabs remain separate.
