# Five-provider deposit setup delivery — 2026-10-29

## Delivered behavior
- Each supported network offers five distinct providers, combining 21 account templates and the existing recommended public profiles. TronGrid public/account access counts once. The two official testing endpoints and nonrecommended ETH dRPC are excluded from this count but remain backward compatible.
- Account metadata distinguishes free, limited backup, trial/paid, paid trace, and plan-dependent access; console/source links and URL/header/full-endpoint setup are explicit. Documentation-qualified does not mean operator-key-verified or currently healthy.
- Selecting any template resets credential drafts. Selecting a dashboard endpoint on an existing row sets sticky replacementEndpoint, blocking old-URL preservation even if the picker is subsequently cleared. TronGrid account mode requires an API key. Unresolved key placeholders are rejected except the existing exact {API_KEY} runtime template with an independent key.
- Existing public defaults, saved pools, encrypted credentials, revisions, cooldowns, scanning cursors and financial validation are unchanged. Template metadata never enters the existing server payload. No migration/backend changes.

## Source/probe boundary
See providers.md for official documents and matrix. One public read on dRPC Bitcoin and Tron each returned 403; neither was promoted to an anonymous default. Account endpoints were not called with real keys, no provider account/subscription was opened, and there were no production database accesses or real payments. Only explicit configuration and complete runtime scanning establish operational readiness. Optional paid/template preference question was unanswered; optional entries are clearly labelled and never purchased or autoenabled.

## Automated verification
- Initial targeted run: 49 passed / 3 failures (old text assertions after label updates). Expanded run: 57 passed / 23 failures (the same old labels plus new async/network fixtures). Follow-up 15 passed / 8 failures isolated the missing Base streams and Tron display label in the new test harness. Fixes kept actual production validators unchanged.
- Deposit suite: 99/99 in 6 files passed. Covers all 21 account templates, exact network/payload, new credential IDs, full endpoint and header mode, distinct provider counts, paid labels, legacy setup, saved-key retention, conflict/cancel and blank dashboard replacement.
- Full Web suite: 127 files / 1262 tests passed with TZ=UTC. Final unresolved-placeholder guard was added afterwards; final deposit suite 99/99 passed again, no full-suite rerun after that local guard.
- Final lint passes. Typecheck passed; final exact-release production build includes TypeScript compilation and passes.
- Production policy 15/15; coverage 23/23 and all thresholds pass.
- Workspace build before the final guard passed; existing budget 2,809,597 / 2,600,000 bytes fails. HEAD plus exact task Web files (excluding agent WIP) final build passes; final budget 2,804,007 / 2,600,000 bytes still fails. Baseline exact version was 2,795,023, so this change adds 8,984 bytes. Thresholds were not loosened.
- git diff --check and task context validation pass. No Rust code was changed; prior scanner/database checks are not presented as newly run here.

## Browser verification
Ego TaskSpace 13, localhost Vite port 3051, actual React/Semi components and isolated in-browser API fixture; no production login.
- 1728px: all five Ethereum choices, paid labels, add Google full-endpoint template, blank URL disables save, valid URL enables it; original channel retained.
- 1280px: Base, Tron, Bitcoin, Solana each reports five provider choices; account selector contents and original public defaults correct; no document overflow; cancelling untouched templates does not save.
- 320px: Google addition saves successfully, confirmation x16 width288, no document overflow. Screenshot inspected.
- 390px: forced409 preserves full endpoint and operation reason, confirmation x16 width358, no overflow; cancel/discard leaves saved dataset absent. Screenshot inspected.
- 390px read-only: zero configuration/write buttons; no overflow.
- Browser locator errors (aria-label versus aria-labelledby, stale ref after viewport change, and ambiguous readonly label) were inspected and corrected without changing production code. They are not counted as passed checks until the final assertions succeeded.
- Screenshots: /tmp/deposit-five-confirm-320.png, /tmp/deposit-five-solana-1280.png, /tmp/deposit-five-conflict-390.png.

## Release scope
Update the admin frontend build. Existing channels are retained until an administrator explicitly adds/replaces and confirms. Own provider credentials/plan permissions are required; this is not five working free anonymous defaults. No deployment, remote push, CI verification or actual sustained RPC throughput validation in this task. Unrelated agent-management WIP is preserved. Browser task finished; task Vite/export cleanup is recorded in progress.
