# Admin v2 gaps and selected approach

- Existing review and project lifecycle already delegate to v2. Admin per-order distribution already uses the v2 allocation transaction; no new financial allocation policy is required.
- Admin creation is legacy-only. Add an explicit v2 creation endpoint and UI sharing IssuancePolicyFields; use existing active asset identity, atomic project/pair/wallet creation and an admin-bound replay receipt.
- New v2 holds and trade quotes are independent of asset_lock_positions / asset_unlock_records. Existing links cannot reveal them. Add read-only holdings, executed sells and operation history, with existing locks/unlocks/audit permissions respectively.
- Existing purchase list does contain v2 purchase orders but is mislabeled. Rename and show v2 held semantics.
- Admin project DTO already contains issuer_agent_id and actual_purchase_at but omits scheduling data. Extend read projection with scheduling/issuer/pair identity; immutable policy stays authoritative.
- Market drafts currently allow any matching asset pair and legacy issuance-price seeding. For v2 constrain to the fixed pair and approved initial seed, and forbid publication before actual purchase; keep current history and generator semantics.
- Legacy admin APIs, old locks/unlocks and per-order settlement remain compatible. No auto-allocation, forced sale, historical backfill or production operation is added.
- Verify real isolated MySQL reads/creation/replay/audit rollback, existing lifecycle tests, permission parity/OpenAPI, strict frontend contracts and responsive real-controls browser flows.
