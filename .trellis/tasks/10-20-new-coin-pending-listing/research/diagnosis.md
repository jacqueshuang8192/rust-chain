# Deployed KHB diagnosis

Observed at 2026-10-20 01:26 Asia/Hong_Kong.

Page: https://hippo.cllbmz.kdns.fr/#/products/new-coins/KHB

Public GET: https://hipoex.cllbmz.kdns.fr/api/v1/new-coins/KHB

HTTP 200, Date: Mon, 19 Oct 2026 17:26:39 GMT, cache status DYNAMIC.

Selected response fields:

```json
{
  "id": 2,
  "symbol": "KHB",
  "lifecycle_status": "distribution",
  "status": "active",
  "reserved_supply": "100.000000000000000000",
  "allocated_supply": "0",
  "remaining_supply": "99999900.000000000000000000",
  "issuance_policy": {
    "version": 2,
    "timezone": "Asia/Hong_Kong",
    "subscription": {"automatic": true, "at": 1792425840000},
    "distribution": {"automatic": true, "at": 1792426500000},
    "purchase": {"automatic": true, "at": 1792426800000},
    "listing": {"automatic": false, "at": 1792529760000}
  }
}
```

The fresh browser shows waiting distribution and highlights its third stage.
Only one project GET was issued while holdings continued refreshing. Public
responses do not expose pending order count or the last scheduler error, so do
not claim access to those fields. Reserved supply with zero allocation is
consistent with outstanding distribution obligations; no production orders
or administrative actions were read or changed.

Code evidence:

- `src/modules/new_coin/infrastructure/issuance.rs`: `advance` blocks purchase
  and listing on unsettled subscriptions; actual state drives events.
- `src/modules/admin/infrastructure/new_coin_settlement.rs`: pending manual
  orders or positive frozen amounts block the transition.
- `mobile/src/views/NewCoinsView.vue`: distribution used pendingListing and
  appeared after purchase in the filter order.
- `mobile/src/views/NewCoinDetailView.vue`: initial-only load; legacy third
  milestone used pendingListing.
- `mobile/src/components/TokenTradingPanel.vue`: refresh concerns holdings only.

Frontend changes must address those presentation/refresh issues without
bypassing settlement or automatically allocating anything.
