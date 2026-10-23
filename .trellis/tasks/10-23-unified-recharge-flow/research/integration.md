# Recharge integration decisions

- On-chain admission now depends on scanner readiness, explicit chain asset mappings, enabled per-network addresses and platform asset policy. Legacy network/inventory allowlists no longer control deposits.
- `deposit_network_configs` remains the withdrawal directory. Its Admin editor moves to withdrawal networks; quote/admission behavior and stored group fields are preserved.
- Migration 0160 removes the shared-address foreign key to `deposit_address_pool`, gives the shared table its own auto-increment key and restores order references. Existing shared IDs remain stable. New imports never create legacy inventory rows.
- The test-environment decision removes permanent address allocation, legacy observation credit and legacy Admin inventory CRUD. Historical records are retained; old gateway deposit receipts are dead-lettered. Withdrawals still use their existing gateway flow.
- Direct imports accept 1..500 addresses with a network and audit reason. Canonicalization and batch validation precede a network-locked transaction; duplicates do not reactivate disabled addresses. Import and audit commit atomically; additions fence stale scans and reset readiness.
- Scanner `enabled` controls new-order admission. A configured endpoint continues scanning even when admission is disabled; settlement checks endpoint presence and revision under the network lock.
- Two Admin recharge navigation entries host independently authorized routed tabs. Address-only operators read the dedicated address endpoint without requesting node settings. Old UI URLs redirect; retired write APIs return 404.
- The five chain parsers and 30-minute matching rules remain from the prior timed-deposit task under `.trellis/tasks/archive/2026-10/10-23-timed-chain-deposits/`. This task does not claim a live-chain transfer validation.
