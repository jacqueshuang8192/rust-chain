# Tron free-channel delivery — 2026-10-29

## Delivered scope
- Five distinct free providers: PublicNode, TronGrid, Ankr, Alchemy, Chainstack. Adds one public provider beyond the previous catalog and qualifies existing account entries with explicit recurring-free plan evidence. QuickNode retained as optional trial/paid (six total choices), never counted as free.
- New immutable tron-publicnode anonymous profile at https://tron-rpc.publicnode.com, 1000ms request interval. Existing pools remain unchanged until explicit add/save; append retains original private keys/priority. Native Tron HTTP scanner and TRC20-only financial evidence rules unchanged.
- Four account templates disclose quota units, official plan link and verification date; browser-only metadata does not enter DTO. TronGrid anonymous/account access counts once. An older server catalog reports four qualified free choices instead of inventing a fifth. No migration, purchase or account signup.

## Primary-source and live-probe evidence
See providers.md and public-probe.json. Five read-only PublicNode requests returned 200; genesis mainnet, USDT decimals six, recent head and complete 496/496 receipt sample with matching height/time passed. No true operator account key, production database, paid access or on-chain payment was used. No sustained throughput or all-history guarantee. GetBlock excluded because current complete native receipt method was not established.

## Automated validation
- Web deposit suite: 6 files / 101 tests passed.
- Full Web: 127 files / 1264 tests passed under TZ=UTC. Final Alchemy pricing copy was subsequently corrected from 25 to 15 req/s after rereading official pricing; final preset tests 12/12, lint and exact build/typecheck passed. No logic changed after the full suite.
- Web typecheck and lint passed; production-policy 15/15 and coverage 23/23 with all thresholds passed.
- Exact release export (HEAD plus task Web files, excluding unrelated agent-management WIP): production build/typecheck passed. Existing JS budget still fails at 2,805,753 / 2,600,000 bytes; prior baseline 2,804,007, delta +1,746. No threshold changes.
- Backend preset tests: 2/2, including actual isolated MySQL 8.4 migration/save/key/cooldown/fencing/rollback test; new Tron pool saved with two anonymous presets. Initial wrong filter matched zero tests, then rerun with correct preset_tests filter actually executed both; zero-test run is not counted.
- Tron scanner/provider tests: 4/4; backend architecture 11/11; production Clippy with -D warnings and cargo fmt --check passed. Scope is catalog-only; no scanner algorithm or API schema changes.
- git diff --check and task context validation passed. 30 protected pre-existing files verified unchanged before final bookkeeping.

## Browser verification
Ego TaskSpace 14, Vite localhost:3052, real React/Semi components against isolated in-browser API fixture; no production login.
- 1728px: catalog shows six choices/five free, Ankr quota and source link visible. 1280px: no page overflow.
- 320px: existing private channel plus TronGrid/PublicNode defaults save successfully; default PublicNode interval 1000ms, confirmation x16/width288, no page overflow.
- 390px: forced409 keeps PublicNode draft and reason, no saved state; confirmation x16/width358. Cancel and discard do not save. Read-only mode exposes zero write buttons, no overflow.
- Ego CDP viewport overrides reset across invocations; initial screenshot named 320 was actually desktop. Detected via reported innerWidth and screenshot, reran all narrow actions with override in the same invocation and replaced screenshot. Only final 320/390 results counted.
- Screenshots /tmp/tron-free-catalog-1728.png, /tmp/tron-free-confirm-320.png and /tmp/tron-free-conflict-390.png inspected.

## Rollout
Build/update API, worker and admin together, then explicitly add defaults or own account credentials under Tron and confirm save. Existing installations already on 0168 need no new migration. Runtime health and actual crediting remain authoritative. No push or deployment in this task.

Cleanup completed: Ego TaskSpace 14 finished, Vite port 3052 stopped, isolated MySQL container stopped/auto-removed; exact-release export removed after final successful build. Logs/screenshots remain in /tmp/tron-free-*.
