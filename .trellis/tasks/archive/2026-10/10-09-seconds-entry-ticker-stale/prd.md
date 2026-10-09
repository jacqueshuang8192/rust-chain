# Seconds Contract Cached-Price Opening And Settlement

## Goal

Adopt the user's requested seconds-contract pricing policy: snapshot the
server's current cached price at opening without the 60-second age rejection,
and use cached-price settlement to avoid waiting for event-time history.
The user confirmed opening and settlement should both consume Redis.

## Known Facts

- The user reports `VALIDATION_ERROR` while opening a seconds-contract order.
- Follow-up clarification: orders eventually settle, but with a long delay.
  Diagnose latency rather than assuming permanently stuck orders.
- `cached_entry_price` rejects Redis ticker observations older than 60 seconds.
- The existing contract rejects old opening quotes and settles from immutable
  event-time MySQL history. The user's new policy explicitly changes these
  requirements for seconds contracts; it is not merely a storage optimization.
- The user accepts a cached price regardless of its age and explicitly requests
  removal of the seconds-contract 60-second opening protection.
- Opening prices remain server-derived. Existing idempotent orders must replay
  their original price and funding without reading a replacement quote.
- The workspace contains unrelated client chart, theme and Pencil changes.

## Requirements

- Trace ticker observation parsing, external feed updates and Redis ownership.
- Explain settlement latency from the event window, scan scheduling, historical
  tick availability, fixed retry delay and client reconciliation.
- Remove the seconds-contract opening ticker age rejection. Preserve positive
  valid-price and cache-availability requirements; never invent a missing price.
- Snapshot the server-read opening price exactly. Do not accept an arbitrary
  client-supplied price or claim an old observation is fresh.
- Proposed settlement rule: read the server's cached price when the due order
  is processed, ignoring observation age, and persist the original timestamp
  and price evidence with the result.
- Apply Redis settlement to due `opened` orders, retaining their original entry
  price. Do not rewrite completed orders or automatically reopen manual review.
- Keep explicit historical-evidence recovery for existing manual-review orders.
- Remove the 5-second event-window wait from automatic cached settlement and
  shorten its fixed 60-second retry and default scan interval to 1 second.
- Preserve existing market cache CAS and cache-before-broadcast sequencing.
- Keep these pricing changes confined to seconds contracts.
- Preserve order funding, settlement evidence and idempotency semantics.
- Add focused regression coverage and record verification in PROGRESS.md.

## Acceptance Criteria

- [x] Positive cached opening prices are accepted regardless of age.
- [x] Missing, malformed or non-positive cache prices still fail before debit.
- [x] Original order prices and funding remain unchanged on idempotent replay.
- [x] Existing-order settlement-policy treatment is confirmed and implemented
      without silently rewriting completed orders.
- [x] The selected settlement price/evidence and wallet/ledger/result commit
      atomically, with no duplicate settlement on retry or restart.
- [x] Focused tests and backend checks pass after the change.
- [x] Runtime limitations and rollout steps are explicitly recorded.

## Open Questions

- Runtime root-cause investigation remains unverified: local API/Redis are
  absent and local configured MySQL access is denied.

## Out of Scope

- Client-selected arbitrary execution prices, inventing prices or refreshing
  original observation timestamps without new provider evidence.
- Removing freshness checks from other trading products.
- Rewriting settled orders, existing entry prices or historical payouts.
- Production database/configuration mutations, deployment, commits or pushes.
- Unrelated frontend, chart, theme or market-engine refactors.

## Technical Notes

- Read `.trellis/spec/backend/seconds-contracts.md` and
  `.trellis/spec/backend/market-provenance.md`.
- Error location: `src/modules/seconds_contract/infrastructure.rs`.
- Initial diagnosis and verification: `research/diagnosis.md`.
- Confirmed implementation boundary: ordinary opened orders consume Redis;
  completed/refunded/cancelled/manual-review orders are not automatically changed.
