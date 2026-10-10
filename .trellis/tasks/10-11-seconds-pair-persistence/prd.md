# Persist Seconds Pair Selection

## Goal

Keep the chosen seconds product across PWA refresh and page re-entry.

## Confirmed Route Extension

User approved `/seconds/BTC_USDT` on 2026-10-11. URL selection overrides memory;
bare `/seconds` restores the last product and replaces the address. Picker
changes use replace, not push. Unknown/unavailable symbols fall back to a valid
product. Keep `/seconds/history` and `/products/seconds` compatibility.
Symbol-only navigation must reuse the live SecondsView instance and preserve
private order/result tracking. Route accessibility must complete without waiting
for a transition that no longer occurs. Add router and browser regressions.

## Requirements

- Persist only the product identity, independently of spot/contract context.
- Resolve against freshly fetched products; removed products fall back to the first.
- Empty lists and failed requests must not erase the saved preference.
- Read the current selection after awaiting products, preserving a user's newer choice.
- Storage unavailable/corrupt must not break trading; retain in-memory state.
- Do not persist stake, direction, prices or order confirmation state.

## Verification

Behavior tests for refresh, reordered/removed/empty products, storage failures and
selection during a pending request. Run the mobile release gate.

## Scope

Mobile selection, router and route-shell lifecycle only; no backend or service-worker changes.
Existing unrelated changes remain untouched. No deployment or remote push.
