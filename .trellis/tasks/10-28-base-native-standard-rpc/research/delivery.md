# Delivery: Base native deposits without trace RPC

## Final behavior

Base native ETH uses `eth_getBlockByNumber(height,false)`, batches of at most 25
`eth_getTransactionByHash` calls, and `eth_getTransactionReceipt` for candidate
payments. It never calls debug/trace methods or requests hydrated block bodies.
Only successful outer transfers to watched addresses produce native evidence.
ERC20 scanning and Ethereum internal-transfer coverage remain unchanged.

Responses are bound to the requested transaction hashes, block hash/height and
positions. Batch response IDs are validated and reordered. Receipt status and
participants must match. Missing/malformed/inconsistent results reject the block.
The original native root index `0x80000000` preserves existing receipt replay
identity. Four-block maximum rounds, durable pacing/cooldown, network revision,
leases and all settlement invariants remain active.

No schema migration, production configuration change, cursor reset, deployment,
private-key handling or real-money transfer was performed. Operator help and the
operations document clearly disclose unsupported internal ETH payments.

## Validation

| Check | Result |
|---|---|
| New Base and batch fixtures | 41 executed and passed, including legacy root evidence equality |
| Scanner suite without DB configuration | 56 executed and passed; 2 existing DB cases explicitly skipped |
| `cargo test --lib timed_deposit -- --nocapture` with isolated MySQL | 63 passed initially; provider case rejected the accidentally chosen fixture DB name before connecting |
| Corrected provider DB rerun | 1/1 passed using `timed_deposits_test_ha`; all 64 selected tests have executed successfully across the run and corrected rerun |
| Isolated DB boundaries | Dedicated temporary MySQL on loopback port 13369; separate accounting/snapshot, recovery and provider databases; server stopped and fixture directory removed |
| `cargo test --test backend_architecture` | 11/11 passed |
| `cargo clippy --lib --bins -- -D warnings` | Passed |
| `cargo check --all-targets` / `cargo fmt --all -- --check` | Passed |
| Initial admin deposit tests | 3 files, 40/40 passed |
| Web full suite | 126 files, 1212/1212 passed with TZ=Asia/Hong_Kong |
| Web typecheck / lint / production-policy / coverage / build | Passed; policy 15/15, coverage 23/23 |
| Web JS budget | Existing failure remains: 2,785,289 > 2,600,000 bytes; +192 bytes over pre-task working-tree baseline, no budget relaxation |
| Real Nodies compatibility | 25-call batch plus a successful sample transfer receipt passed identity checks; 100-call batch returned HTTP 429 |

The final implementation was reviewed against the backend timed-deposit and
quality contracts. No browser visual session was needed for the two operator
copy edits; component tests and the Web build were used. No production worker
or sustained-rate test was run. This is not a claim that the live scanner has
recovered or that the existing RPC plan has sufficient ongoing capacity.

## Rollout

Deploy the updated backend and matching operator copy. Existing cooldowns remain;
wait for expiry, then verify last_error clears, cursor progression catches up and
scan_fresh becomes true. Base internal-only ETH transfers, including not-yet-seen
historical ones, are outside this explicitly selected direct-transfer mode.
Already stored internal evidence is retained.

Do not reset the cursor or skip historical ranges to conceal throughput trouble.
If the plan remains throttled or cannot keep up, adjust provider capacity based
on observed quota/latency. Batching still consumes individual method-call quota.

## Scope isolation

The pre-existing agent-management layout changes, its specifications/fixtures,
other unrelated tasks and `.vite/` are outside this delivery. Only Base scanner,
its tests/help/spec and the related diagnosis/implementation records belong in
this task's commit. No remote push is part of this follow-up request.
