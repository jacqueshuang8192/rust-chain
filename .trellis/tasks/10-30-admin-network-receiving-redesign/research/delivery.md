# Delivery and verification

## Changes
- Full-width network workbench: desktop network navigation, mobile select, readiness/pending overview, receiving/RPC/progress tabs.
- Compact setup checklist and responsive currency/address sections; existing DTOs and form semantics remain.
- Block network/tab navigation during editor/address confirmation; recover from failed reads via enabled refresh.
- Deposit-only confirmation width/wrapping and SideSheet title fixes. Optional ConfirmAction.modalClassName preserves other callers.

## Validation
- Full Web suite: 127 files, 1267 tests passed (TZ=UTC).
- After final modal/scoped wrapping changes: deposit suites 104 tests passed; shared confirmation suite 6 passed after correcting the test to check Semi's wrapper class, not inner modal class.
- Final lint passed; exact delivery export (HEAD plus this task's Web files, excluding agent-management WIP) build/typecheck passed.
- Production policy: 15 passed; coverage: 23 passed and thresholds passed. git diff --check passed.
- Exact delivery JavaScript: 2,810,324 / 2,600,000 bytes, existing budget failure retained (previous release 2,805,753; +4,571 bytes). No threshold changes.
- Ego TaskSpace 15 with actual AdminLayout/React/Semi and local simulated API: 1728 network workbench; 1280 channel and empty/addresses-only views; 320 address save and full long-address preview, recovery title/close bounds; 390 read-only and RPC 409 reason/15-confirmation draft retention then discard. No document overflow; modal bounds 288px at320 and358px at390. Error mode exposes retry without a stale overview. Screenshots inspected.
- Initial tests queried a now-hidden desktop combobox; updated to visible navigation. Initial address confirmation overflow found and repaired. Initial address-only shell fixture path mismatched authorized route; corrected fixture and reverified. These initial failures are not counted as passes.
- No production RPC, credentials, real funds, database migrations or backend changes.

## Artifacts
Screenshots and logs: /tmp/network-workbench-*.
Browser space finished. Temporary release export and Vite cleanup performed at delivery.

## Remaining limitation
Existing JS bundle budget still fails; runtime upstream RPC health was not tested by this UI change. No push/deploy in this task. Unrelated agent management files and progress remain local.
