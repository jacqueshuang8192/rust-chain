# Base standard-RPC implementation and provider evidence

## Decision

Scope is Base only. Native deposits use the block's transaction hash list,
25-call JSON-RPC batches of `eth_getTransactionByHash`, and individual successful
`eth_getTransactionReceipt` evidence for positive direct transfers to watched
addresses. ERC20 parsing and Ethereum trace/internal-transfer coverage remain
unchanged. No native balance-difference heuristic or trace fallback is used.

The user's request followed the explicit explanation that ordinary direct
transfers exclude internal contract payments. Operator setup help and the
recharge operations guide disclose that limitation, including historical
unobserved payments. Existing recorded internal evidence is not deleted.

## Why this exact request shape

Read-only tests on the user-supplied Nodies endpoint on 2026-10-28 established:

| Request | Observed result |
|---|---|
| `eth_getBlockByNumber(height,false)` | HTTP 200, transaction hashes |
| `eth_getBlockByNumber(height,true)` | HTTP 200 / RPC -32601: fullTransactions=true unavailable on this plan |
| `eth_getBlockByHash(hash,true)` | Same explicit full-transaction entitlement restriction |
| `eth_getBlockReceipts(hash)` | HTTP 200 / RPC -32601: method unavailable on this plan |
| `eth_getTransactionByHash(hash)` | HTTP 200, transaction identity/from/to/value present |
| `eth_getTransactionReceipt(hash)` | HTTP 200, identity/from/to/status present |
| 3-call transaction batch | HTTP 200, complete replies |
| 25-call transaction batch | HTTP 200, all returned transaction/block/index identities match |
| Receipt for a positive transaction from that batch | HTTP 200, status 1, all identity and participant fields match |
| 100-call transaction batch | HTTP 429, plan throughput restriction |

The chosen batch limit is 25. Each HTTP request still goes through the existing
provider/key gate and its persistent interval. Native Base rounds cap the
existing adaptive block batch at four. This limits request bursts and round
duration; it does not create quota or guarantee that this account can keep up
with sustained Base traffic. Deployment must verify actual catch-up and freshness.
The 25-call probe is a sampled compatibility check, not a sustained load test.

`rpc-probes.json` retains sanitized request modes, status and identity checks.
No API key, credential URL, raw transaction body or raw receipt is recorded.
One earlier 100-call probe reported only an HTTPError; the later controlled
probe established HTTP 429. Do not infer its unspecified earlier status.

## Correctness and replay

- Batch replies must contain every requested numeric ID exactly once and
  declare JSON-RPC 2.0; replies are reordered by ID before binding each result
  hash to the block's ordered hash list. No partial batch result escapes.
- Every transaction is checked for unique hash, block hash/height and exact
  position, canonical sender/recipient and a valid unsigned hexadecimal value.
- A native receipt requires matching transaction hash, block hash/height/index,
  sender and recipient. Status 1 succeeds; 0 skips; other/missing status fails.
- Zero, self, unwatched, creation and internal-only transfers produce no native
  evidence. Missing or mismatched data fails the block and retains progress.
- Native `event_index=0x80000000` matches the legacy callTracer root. The fixture
  compares the full receipt against the retained Ethereum root parser with the
  network identity normalized to Base; the existing SQL uniqueness and replay
  checks therefore see the same direct-payment evidence.
- No migrations, API changes, cooldown resets or production writes.

## Primary reference material

- Ethereum execution API schemas: https://raw.githubusercontent.com/ethereum/execution-apis/main/src/eth/block.yaml
  defines the hydrated-transactions boolean and standard block-receipt method;
  availability is account-specific as the probes above demonstrate.
- Receipt status: https://raw.githubusercontent.com/ethereum/EIPs/master/EIPS/eip-658.md
  specifies zero for failure and one for success.
- JSON-RPC batch ordering/identity: https://www.jsonrpc.org/specification#batch
  (implementation rejects invalid identities and never relies on response order).

## Validation

Final commands, counts and limitations are recorded in `delivery.md` after gates
finish. The initial focused scanner run passed 56 executed tests; two existing
database cases skipped until isolated MySQL was started for the subsequent run.
