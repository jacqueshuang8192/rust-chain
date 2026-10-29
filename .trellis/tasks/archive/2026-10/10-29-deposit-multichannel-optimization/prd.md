# Multi-channel deposit queries and administration

## User request

Optimize multi-channel queries for every supported network and improve the admin configuration page. Existing scope: Ethereum, Base, Tron, Bitcoin and Solana.

## Accepted scope and default

Use priority-based sequential failover, with one eligible credential per independent provider before returning to secondary credentials. An optional strategy question was asked; absent a preference, proceed with this conservative default. Do not add concurrent duplicate full scans.

## Deliverables

1. Fair, bounded provider attempts shared by all five networks. Preserve durable pacing, provider/key/stream cooldowns, per-attempt identity checks, leases, revisions and financial error stops.
2. Use lighter anchor-only Bitcoin and Solana responses; retain complete transaction evidence for actual deposit scanning. EVM and Tron already avoid unnecessary receipt/trace work for anchors.
3. Expose existing Base provider stream failures in the admin snapshot, without exposing endpoints or secrets and without a schema migration.
4. Separate scan progress from channel configuration/status. Summarize eligible channels and key slots, show cooldown scope, and make channel editing easier with collapsible per-channel forms and an explicit priority order preview. Preserve existing permissions, drafts, reasons, conflicts and polling behavior.

## Non-goals

No ledger changes, cursor skips, weakening of Ethereum trace evidence, guessed quota increases, new provider credentials, production operations or automatic pushes. Existing unrelated agent-management work stays intact.

## Plan

- [x] Implement shared failover and anchor optimizations with regression tests.
- [x] Add stream-failure response and strict frontend parsing with contract tests.
- [x] Implement and verify admin layout/editor interactions.
- [x] Run isolated database, Rust/Web gates, browser checks; update specs and progress.

## Acceptance

- Four failing keys in one provider cannot prevent another eligible provider from being attempted; quick failures across more than four channels can reach a later healthy channel within the unchanged round deadline.
- Provider-wide cooldown excludes sibling credentials; key cooldown does not exclude other keys. Chain/continuity/accounting failures still stop the scan.
- Anchor response requests are light while normal block requests remain complete.
- Base native-only cooldown is visible separately from token scanning. Missing stream-state fields in an older backend are unknown, not healthy; malformed new fields are rejected.
- Admin desktop and 320/390px layouts remain usable, preserve independent credentials and pinned revision through collapse/save/cancel/409, and never claim readiness from a channel count alone.

## Validation

Targeted mock RPC and isolated MySQL tests (no optional skips counted), API schema and architecture guards, Rust static checks; Web targeted/full tests, type/lint/policy/coverage/build/budget; local browser fixtures with screenshots. Record existing unrelated failures explicitly.
