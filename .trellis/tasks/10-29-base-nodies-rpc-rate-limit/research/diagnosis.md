# Nodies Base HTTP 429 diagnosis

Checked on 2026-10-29. No live credential probe or production access was performed.

## Official documentation

- https://www.nodies.app/pricing — pricing is per request rather than compute units. The Free plan lists 4 million requests/month and a low rate limit; Starter and Pro list 10 million and 50 million respectively with higher rate limits. The retrieved page text does not specify numeric RPS limits or JSON-RPC batch accounting. These are published plan descriptions, not evidence of this user's entitlement or remaining balance.
- https://docs.nodies.app/rpc-services/pricing-plans — monthly allowances and paid overage billing. Do not assume every paid account stops when its included allowance is exceeded.
- https://docs.nodies.app/application-guide/creating-and-managing-your-workspace/api-keys — multiple keys are supported in a workspace. Documented authentication uses URL `apikey` or the `x-api-key` header. It does not promise that additional keys multiply throughput or monthly allowance.
- https://docs.nodies.app/application-guide/creating-and-managing-your-workspace/stats — workspace metrics include status classes and usage by network.
- https://docs.nodies.app/application-guide/creating-and-managing-your-workspace/recent-logs — latest 100 requests include timestamps, network, method and status.
- https://docs.nodies.app/supported-api-methods/ethereum/eth_getblockbynumber — `true` requests full transaction objects, `false` requests hashes. This Ethereum API reference is not proof of Base entitlement for a particular plan.

## Current code evidence

1. `scanners/evm.rs` reads the complete block hash list and calls `rpc_batch("eth_getTransactionByHash", ...)` for every transaction before selecting watched recipients. A block with N transactions therefore needs N transaction subcalls even if no deposit is present. Receipts are requested only for positive, non-self outer transfers to watched addresses.
2. `scanners.rs::rpc_batch` chunks at 25. `post_operation` acquires the persistent provider/key gate once per HTTP batch. The gate has no request-weight or batch-size parameter. It controls envelope spacing, not a declared RPC-call budget. A successful small batch is not a sustained-capacity check; the documentation reviewed does not explicitly define how Nodies bills batches.
3. Native scans can process up to four blocks concurrently (`buffer_unordered(4)`). The gate is shared across native and token streams, but the two streams have independent progress. A selected key's actual HTTP 429 cooldown is shared; a method-specific native error is scoped. Consequently USDC can still be affected by a genuinely exhausted shared key.
4. The gate honors Retry-After and persists cooldowns. A failed scan retains its previous checkpoint. Retrying repeats already-read blocks/subcalls in that uncommitted round, so repeated throttling can increase consumption without progress.
5. Provider/key intervals allow 100–2000 ms and default to 250 ms. Merely changing this interval to a much slower value can hit the existing 35-second attempt deadline before a large block completes. A future weighted-budget fix must address bounded work/deadlines and verify actual catch-up; it must not silently skip history or mark a lagging scan ready.
6. For endpoints without `{API_KEY}`, non-Tron requests currently use Bearer authentication. Nodies only documents `apikey`/`x-api-key`. The existing URL template mechanism correctly inserts the selected key into the query, so template configuration is a supported remedy without broad authentication changes. If a literal key is embedded in the endpoint, rotating separately configured key slots does not replace that embedded query value. The current deployment's endpoint mode was not inspected.
7. `API Key 已配置` is a local presence flag, not a successful-authentication or paid-plan assertion. The HTTP 429 text is sanitized fixed guidance; it cannot distinguish current RPS throttling, exhausted allowance or another provider policy by itself.

## Earlier evidence, not a new probe

The archived 2026-10-28 native-RPC research includes a sampled block with 225 transactions, successful 25-call batches, a 100-call HTTP 429, and explicit entitlement rejections for `fullTransactions=true` and `eth_getBlockReceipts`.

Source: `.trellis/tasks/archive/2026-10/10-28-base-native-standard-rpc/research/rpc-probes.json` and `implementation.md`.

Therefore it would be incorrect to switch blindly to full blocks or claim that waiting/adding keys will permanently solve this issue. The prior trace change fixed method compatibility, not sustained throughput. Migration 0166 intentionally preserves genuine quota errors.

## Remedies and decision boundary

- Verify account plan, effective RPS, this month's usage and whether other consumers share the account/workspace. Compare this message with fresh Nodies Recent Logs to distinguish a new rejection from a retained channel error.
- Verify template-based URL authentication or documented `x-api-key`; never infer validity from the local presence flag. Do not request or log credentials.
- If enough quota exists but bursts cause 429, implement a declared call budget and smaller/adaptive batches, with a work/deadline design that can finish blocks. Preserve durable gates, lease fencing, atomic block evidence and cursor safety.
- If total capacity is insufficient, lowering request rate alone cannot make a full-chain native scan catch up. A provider/plan permitting full transaction blocks can reduce the native lookup count substantially, while retaining success-receipt checks and direct-transfer-only semantics. Actual method eligibility must first be verified.
- Retain token/native independence; do not erase a shared 429 to make the token stream appear healthy. More key records are not evidence of independent quota.

## Verification and remaining uncertainty

`cargo test --lib transaction_batches_are_bounded_and_restore_request_order -- --nocapture`: 1 passed, 0 failed, 602 filtered out. The existing fixture verifies that 101 transaction subcalls become five HTTP envelopes and restore reply order. It does not test Nodies billing, production timing, sustained throughput or successful deposits.

No business code changed. The current account tier, production request interval, endpoint mode, provider log timestamps and actual quota cause remain unknown. A concise account-information question was sent to the user; no answer has arrived at the time of this record.

## Follow-up and implemented remedy

The user subsequently confirmed Free Plan, sufficient remaining monthly allowance,
and no displayed numeric RPS. These answers supersede the initial account-tier
and allowance unknowns above. Production timing, endpoint mode, shared-account
consumption and the provider's precise rejection reason remain unverified.

Isolated MySQL testing reproduced a local request-admission race: a sleeping
request ignored a sibling response's extended deadline. The implementation now
rechecks admission after every wait and learns a smaller per-key transaction
batch after Base batch HTTP 429. This preserves cooldowns and full-block evidence
without claiming a numeric RPS or guaranteed sustained capacity. See
`delivery.md` for validation, deployment order and remaining limits.
