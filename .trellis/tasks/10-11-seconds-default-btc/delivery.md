# Delivery

- Bare and legacy Seconds routes default to BTCUSDT, independent of stored ID
  and API order. Explicit symbol routes remain authoritative.
- Pending picker selections retain their existing race protection; missing BTC
  falls back to the first available product.
- Full mobile release gate passed: 825 tests, zero failures/skips, type checks,
  PWA/Tauri builds, artifact, bundle and source/test-quality budgets.
- Ego TaskSpace 11 verified explicit ETH followed by bare entry and hard reload
  resolves to `/seconds/BTC_USDT` with BTC header. Anonymous only; space closed.
- Local commit only, no push/deployment or installed-device PWA test.
