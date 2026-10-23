# Recharge configuration and scoped funding history

## Configuration

- Replace the implicit global dirty lock with network overview cards, asset/address lists and explicit SideSheet editors. Closing a dirty editor asks to discard locally; save failures keep fields, reason and original revision. Network changes do not mutate a pinned draft. Disable refresh while a save is in flight.
- Settings include a minimal platform asset catalog (symbol, deposit_enabled, status, precision_scale). Network operators need no unrelated asset-management permission to select an existing asset. Catalog is mandatory on full settings responses and absent from address-only reads.
- Native identity presets: Ethereum/Base ETH/18, Bitcoin BTC/8, Solana SOL/9; Tron supports only explicitly configured TRC20. Token contracts and decimals are deliberately entered and reviewed; no unverified contract presets. Existing mappings have immutable identity and explicit read-only explanations, with enabled editable.
- Each network lists all mapped assets simultaneously; addresses are shared by all configured assets. No upper bound of ten addresses.

## Agent funding history

- GET /agent/api/v1/funding/{kind}, kind deposits|withdrawals, response {items,total}. Deposits combine quick and chain orders; withdrawals read existing requests. Detail drawer displays the same scoped row, with no unscoped detail query.
- Common filters: team_scope=all|own|descendants, agent_id (subtree branch), user_id, email literal substring, asset_symbol, network, status, created_from/to, limit/offset. Deposits additionally source=quick|chain and exact order_no; withdrawals use record_id instead. Validate bounded safe IDs, times, page limits and unknown fields before DB access.
- Both row and count queries repeat token-derived materialized path ownership. agent_id only narrows that path. Stable sort created_at DESC, source DESC, id DESC. Out-of-scope filters yield no rows/count.
- Amounts remain decimal strings. Requested/quoted coin amount, recorded credited coin quantity, fee and quick fiat quote/currency are separate fields. Quick paid maps to credited; chain effective status follows deposit events (reversed/manual_review) and clock expiry. Do not infer balances or mutate orders while reading.
- Projection allowlists omit provider payment URLs/payloads, callback secrets, idempotency keys and internal withdrawal review/security details. Existing manual recharge requests remain separate.

## Verification

- Focused React tests for real controls, multi-asset creation, native defaults, immutable fields, cancel/discard, conflicts and permissions; funding rows, filters, precision and stale-session/request isolation.
- Isolated MySQL HTTP tests with root/child/grandchild/sibling/other/unassigned users for rows, count, status, paging, source, precision and no writes. OpenAPI, architecture, types/lint/build and browser desktop/narrow interaction checks.
- Prior global web JS size over budget remains a known baseline; no threshold relaxation. No production configuration or live-chain transfer is required to verify these UI/read-only changes.
