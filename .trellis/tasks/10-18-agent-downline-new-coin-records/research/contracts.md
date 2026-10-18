# Repository findings
- Existing team orders implement server-derived subtree scope in a shared rows/count builder. New business kinds can reuse it without a new endpoint family or schema migration.
- Subscriptions use new_coin_subscriptions; purchases use new_coin_purchase_orders and settle after listing. Purchase status is pending/locked/available.
- Pre-manual-distribution code (commit 6f839e1 parent) persisted subscription available for instantly delivered historical orders. Do not reject this value. Migration0121 deliberately leaves settled/refunded historical values null.
- New fields project_id and symbol only narrow authenticated scope; project issuance ownership never grants investor visibility outside the subtree.
