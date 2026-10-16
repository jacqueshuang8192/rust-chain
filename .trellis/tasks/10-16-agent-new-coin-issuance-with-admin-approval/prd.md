# Agent new-coin issuance applications

## Requirements
- Agents submit a new coin symbol/name, precision, supply, quote asset, issue price, planned listing time and reason.
- Submission only creates an immutable pending application; it never creates a public asset/project.
- Admin reviews with a required reason. Approval atomically creates the asset (deposits/withdrawals/margin transfers disabled) and a preheat project using the existing project lifecycle. Reject creates neither.
- Approved means published as preheat, not automatically subscribed/listed, externally minted or traded.
- Agents see their own portal-account applications, including time, status, complete input, review reason/time and published project ID. Parent/sibling/other accounts cannot read their records.
- Server derives ownership from authentication. Submission idempotency, row-locked terminal review and transactional audit prevent duplicate publication.
- Admin read/review use independent new_coin.applications permissions. Existing direct administrator project creation remains unchanged.

## Validation
- Actual isolated MySQL route tests: owner isolation, pending invisibility, reject/approval, exact decimals, request replay/conflict, simultaneous reviews, audit rollback and auth/RBAC.
- Web API/component tests for payload, history/filter/detail, late/session responses and review permission. Type/lint/build and existing gates.
- Browser desktop/mobile visual checks using local fixtures, no production writes.

## Delivered and verified
- Agent create, own list/filter/pagination/detail and Admin queue/detail/review implemented.
- Pending is isolated from public resources. Approval atomically creates disabled-funding coin asset and preheat project; terminal replay and audit rollback verified in isolated local MySQL 9.3.
- Same-node other portal accounts cannot read the applicant's records. Existing Admin direct project creation passed its real DB regression.
- Backend: library 526, architecture 11, documentation 1, OpenAPI 16, all-target check/strict Clippy/fmt passed. DB suite 1 comprehensive issuance test plus 1 existing project test executed with DATABASE_URL set; migration checks repeated.
- Web: full 1072 tests passed; final focused suite 8/8 after response-binding regression added. Type/lint/build, policy15 and coverage23 pass. Existing total JS budget remains blocked at 2675592/2600000 bytes; no budget relaxation.
- Browser: actual form submission/review decisions with in-memory fixtures, own approval/rejection history, 1728/1280/390/320 overflow checks, 130->190px pointer column resize. Screenshots inspected. No production writes or real-device claims.
- Rollout: migration0151 before API/Admin; grant independent read/review permissions. No push/deployment.
