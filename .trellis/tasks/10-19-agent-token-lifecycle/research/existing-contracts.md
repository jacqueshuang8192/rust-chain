# Existing contracts and implementation map

## Reuse
- Agent issuance request/review: src/modules/agent/{presentation,application,infrastructure}/coin_applications.rs. Current approval already creates an asset, zero wallets for existing users, preheat project, audit and receipt atomically. Additional rules must join this same transaction and idempotency comparison.
- Admin partial allocation: src/modules/admin/application/new_coin.rs and infrastructure/new_coin_settlement.rs. Freezes entire quote at subscription; final allocated qty consumes original issue-price snapshot and returns difference, with quota release. Existing helper accepts exact quantity, not percentage. Single terminal outcome and idempotent receipt exist.
- Agent team record read: shared static SQL and scope builder in agent/*/team_orders.rs. Visibility follows buyer ownership path. Project creator is intentionally irrelevant.
- Public new coin APIs: new_coin/{routes,application,presentation,repository,infrastructure}. Four stages currently; purchases require listed and the single issue_price. No user sell/buyback endpoint.
- Lock implementation: actual-listing gates in asset_lock_positions, per-source rows and pending unlock records. Fixed/relative unlock uses timestamps. Legacy scanner moves locked to available after maturity/payment; v2 exit must not be handled by that scanner.
- Synthetic market: existing generator strategy and default-market writer/lease/version machinery plus project market drafts. Use actual Redis latest price with source/version/timestamp validation; do not invent quote from chart or fall back to issuance price.
- Existing quote-consume settlement model: convert/application.rs and infrastructure.rs. Persist one-time user-bound quote and use database clock; settlement record and financial journal commit together.

## Gaps and constraints
- Five-stage lifecycle required: preheat -> subscription -> distribution -> purchase -> listed. Historical projects need their original four-stage path.
- Fixed subscription price, purchase market seed and reference-only listing price; stage actual timestamps differ from planned times. No new-coin stage scheduler currently exists.
- Months currently unsupported (only relative seconds). Agent application pins only a single planned listing instant and no timezone identity.
- Configuration is mutable today, while v2 holding fee/lock/cost rules must remain snapshotted per lot. Legacy merged locks cannot be retroactively assigned per-lot buyback cost.
- No fixed unlock fee; existing fees are calculated using allocation-time inputs and paid separately. New buyback fees need sale-time quote/cost and deduction from proceeds.
- User UI: mobile NewCoinDetailView/NewCoinRecordsView + api/core models; PC activity launchpad and User/LaunchpadOrders + backendAdapters. Both must preserve v1 old records and offer v2 exit only for eligible lots.
- Same asset must not be sellable/spendable via another wallet path while a buyback holding still claims it. New holdings need explicit accounting ownership and transactional prevention of double spend, not frontend button restrictions.

## Test matrix
- Price and quantity decimal bounds;50/80/100/0 percent, partial unit rounding explicit, refund conservation.
- A/B and C/D cross-project purchase with read+write scope; children/siblings/unassigned; reassignment during allocation; disabled ancestors.
- Batch duplicate IDs and mixed ownership rejection before writes; item replay, changed ratio conflict, batch crash/retry and competing single/batch.
- Actual stage vs planned time, manual nodes, restart/lease races, missing prior stage, pending distributions block advancement, engine unavailable.
- No lock; purchase/listing actual events; fixed instants across timezones; relative weeks and calendar-month end/DST; no worker-created sale.
- Buyback quote expiry/future/stale source, changed rules after buy, positive profit/loss/no-fee/fixed-fee, fee>=gross, partial quantities/cost basis, concurrent double-sale and audit/ledger rollback.
- Existing v1 subscriptions/purchases/locks and pending issuance approvals retain previous behavior; new schema migrations apply without editing old migrations.

## Confirmed market handoff
Purchase execution uses fresh authoritative platform price; listing enables spot on the existing pair without restarting its generator or history. Issuance holdings stay in reserved wallet accounting until atomic buyback; ordinary spend paths must not consume them.
