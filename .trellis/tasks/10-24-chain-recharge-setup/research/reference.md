# Reference review

Pinned GMPay Edge commit: `5c32e35f8b92625e7f03040dd5b5c2c154ef9af7`.
Primary sources: `docs/zh-CN/PAYMENT_METHODS.md`, `src/features/payment-settings/readiness.ts`, `server/check-method-readiness.ts`, `server/connection-health.ts`, provider-config.ts, catalog.ts and admin pages from https://github.com/GMWalletApp/gmpay-edge . Retrieved read-only via GitHub API/raw files under /tmp/gmpay-edge-reference after web tool returned no readable result; upstream code not executed.

Reuse concepts: separate network/node access, explicit asset identity and receiving addresses; give readiness reasons instead of treating enabled as healthy; credentials belong to the node connection; snapshot historical orders independently.

Do not copy incompatible behavior: fiat/USD conversion and amount locks extending past expiry differ from the accepted exact-coin/30-minute contract. Upstream EVM/Solana scans are address/order-window based and its cursor-gap recovery differs from our complete-block fail-closed scanner. No WSS/Alchemy webhook switch should be offered without real adapters.

Local plan: configuration checklist + per-method eligibility explanations + real persisted scanner progress/freshness from DB + dedicated preset API-key fields. A successful last caught-up scan (not a configuration click or superficial network ping) remains the admission health signal. No new scanner mutation API, migration or private-key handling.
