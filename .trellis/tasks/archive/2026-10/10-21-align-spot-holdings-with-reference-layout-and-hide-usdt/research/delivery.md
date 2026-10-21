# Delivery

- Replaced framed card with reference-style flat asset row and three-column metrics: equity quantity, cost placeholder, latest USDT price. Added secondary estimated equity and preserved available/frozen-locked breakdown.
- USDT excluded only from current-pair holdings. Existing wallet collection, buy balance, Assets page and loading/error/empty states unchanged.
- Cost basis/PnL absent from API; cost displayed as em dash, no invented return or unsupported trading actions. Existing issuance valuation helper provides exact values and expiry semantics; 10-second clock cleaned up on unmount.
- Added actual computed/component behavior tests for filtering, wallet preservation, exact total, expiry/recovery and cleanup. Numeric browser fixture optionally provides BTC+USDT and known valuation.
- Mobile release:gate passed896/896, types, PWA/Tauri Web builds/artifacts/budgets/governance; final rerun includes baseline alignment of price unit. Log /tmp/spot-reference-release.log.
- Ego space4 light/dark320/390/448 actual app fixture: only BTC row, USDT still in order balance, equity2.8, estimated232795.472, available2.5, frozen+locked0.3, latest83141.24; no page/card overflow, zero writes. Screenshots inspected; subsequently moved USDT unit below latest price to align numeric baselines. No other layout changes.
- No production API or native-device checks. Own preview and Ego closed; local commit only, no push/deployment.
