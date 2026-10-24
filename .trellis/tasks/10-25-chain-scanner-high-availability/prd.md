# Multi-provider chain scanner availability

## Goal
Support multiple explicitly configured providers and API credentials per network, with controlled request rates and safe failover. The user observed TronGrid HTTP 429 on getblockbynum even after replacing the key.

## Known requirements
- Multiple channels and API keys manageable in the admin deposit configuration.
- Preserve restart checkpoints, chain identity checks, receipt identity and exactly-once credit.
- Persist cooldowns across worker rounds/restarts; avoid retry storms and repeated requests to limited credentials.
- Never expose keys or private endpoint URLs in responses, audits or logs.

## Confirmed decision
- User chose primary/backup failover, with multiple keys per channel.
- Apply the common configuration model to all existing networks; retain network-specific read-only adapters.
- Up to 10 channels/network and 10 key slots/channel; priority order, enabled flags, channel/key request intervals. Endpoint templates may interpolate URL-encoded {API_KEY}; other endpoints use existing Tron/header authentication.
- Persist transport cooldowns, use provider/key IDs rather than credentials in status. No automatic chain switches or checkpoint resets.
- Existing node becomes a default channel; legacy scanner save remains supported when no provider payload is sent. Nested provider saves fence revisions atomically and preserve omitted secrets by stable IDs.
- All unavailable keeps network unavailable; first successful full attempt revalidates mainnet and persisted block hash before ready status.

## Investigation and plan
1. Map scanner/provider transport and admin configuration contracts; measure current request amplification.
2. Define provider/key persistence and revision-fenced writes; migrate existing single node without losing checkpoints.
3. Implement per-provider/key pacing, cooldown and safe failover with chain validation per selected provider.
4. Add admin provider/key editing and health display using existing controls.
5. Test HTTP failover, restart persistence, mixed-chain rejection, duplicate credit protection, UI and DB contracts; update specifications and progress.

## Non-goals
No automatic purchase of node services, creation of third-party API keys, deployment, cursor skipping or reduced confirmation depth. Multiple keys cannot promise availability or create additional provider-account quota.

## Acceptance
- [x] Multiple providers/keys can be configured securely.
- [x] Recoverable node failures switch eligible providers without skipping blocks.
- [x] 429 respects cooldown and suppresses rapid reuse; pacing controls normal and backlog requests.
- [x] Runtime status and admin validation expose meaningful readiness without credentials.
- [x] Existing config and financial invariants preserved, migrations and focused tests validated.

## Verified slices
- 0161-to-0162 migration copies existing encrypted endpoint/key, repeat migration succeeds.
- Real isolated MySQL plus HTTP mocks: primary 429 + Retry-After, backup key 401, second backup key success; pacing; cooldown survives new scanner instance/round; blank credential save retains cooldown; cross-provider key ID rolls back; all channels fail without cursor advancement.
- 20 timed-deposit tests passed including original reservation/accounting/reorg tests. Admin focused suite 28 passed. Production Clippy passed before final response-read fallback addition; final gate pending.

## Final verification
- 21 timed-deposit tests passed with real isolated MySQL and local HTTP mocks, including final completion-based pacing, configuration-save fencing and Tron query credential preservation.
- Web full suite passed 1197/1197 with TZ=Asia/Hong_Kong; the initial host +03 run exposed an unrelated timestamp assumption, with no unrelated source changes.
- Admin actual API/browser verified new keys/providers, blank-secret replay, interval validation, dirty cancellation and 1728/390/320 layouts.
- Architecture 11/11, OpenAPI 20/20, Web types/lint/policy/coverage/build passed. Existing JS budget remains 2774218/2600000 bytes (+10258); threshold unchanged.
- Final isolated HTTP permission/ownership route 1/1, production Clippy --lib --bins -D warnings, all-target check, fmt, diff and task context validation passed. See delivery.md for release constraints. Implementation verified; no commit/push/deployment performed.
