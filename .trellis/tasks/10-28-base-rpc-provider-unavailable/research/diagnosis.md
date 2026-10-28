# Base RPC capability diagnosis

## Conclusion

The supplied Nodies credential successfully reaches Base mainnet, but its current plan rejects `debug_traceBlockByHash`. The live read-only probe returned HTTP 200 with JSON-RPC code `-32601` and the explicit message `Method not available on this plan: debug_traceBlockByHash`. The user subsequently confirmed the production provider status says `RPC 方法 debug_traceBlockByHash 失败，请检查渠道权限和额度`, matching this failure path.

This is a demonstrated method entitlement restriction on the tested credential at the recorded time, not a blanket claim about Nodies capabilities or all subscription plans. No application change, production configuration change, credential replacement, cursor repair or deployment was performed.

## Read-only verification

Six sequential requests, spaced by at least two seconds, ran on 2026-10-28 between 10:38:05 and 10:38:54 UTC (13:38 Europe/Istanbul). Each used a 15-second HTTP timeout. Sanitized evidence is in `rpc-probe.json`; it contains no endpoint or credential.

| Request | Observation |
|---|---|
| `eth_chainId` | HTTP 200, `0x2105` / 8453 (Base mainnet) |
| `eth_getBlockByNumber(latest,false)` | HTTP 200, block `0x32bb9d6` |
| `eth_getBlockByNumber(head-20,false)` | HTTP 200, block `0x32bb9c2`, 317 transactions |
| `eth_call(decimals)` | HTTP 200, sample WETH decimals = 18 |
| `eth_getLogs` with blockHash/address/Transfer topic | HTTP 200, 25 sample WETH logs, all block identities match |
| `debug_traceBlockByHash` with callTracer, timeout=15s, onlyTopCall=false | HTTP 200, JSON-RPC `-32601`, current-plan method restriction |

The WETH contract `0x4200000000000000000000000000000000000006` was only a capability sample. These calls do not verify the user's production token mappings, history cursor, every block, sustained rate allowance or server egress. No full raw logs or transaction traces were persisted. Public documentation searches yielded no usable authoritative result; the provider response and local source are the evidence for this diagnosis.

## Code path

- `src/modules/wallet/infrastructure/timed_deposits/scanners/evm.rs:106`: presence of a native asset requires `debug_traceBlockByHash` with `callTracer` for each scanned block, including successful internal transfers.
- `src/modules/wallet/infrastructure/timed_deposits/scanners.rs:105`: scanner assets include enabled mappings OR mappings referenced by any existing deposit order. Merely disabling ETH does not eliminate trace requests when historical ETH orders remain.
- `src/modules/wallet/infrastructure/timed_deposits/scanners.rs:401`: a JSON-RPC error records the exact method with a fixed safe message and cools the whole provider, with a 60-second minimum.
- `src/modules/wallet/infrastructure/timed_deposits/scanners/provider_runtime.rs:215`: repeated failures increase cooldown, up to 3840 seconds for this 60-second-minimum RPC error path. Existing cooldown survives ordinary restarts and saves with blank credentials.
- `src/modules/wallet/infrastructure/timed_deposits/scanners/provider_runtime.rs:38`: disabled/cooling providers or keys are excluded. No eligible candidate produces the user's aggregate warning while retaining progress.

The production cooldown timestamp and complete provider configuration were not supplied. The user-supplied method error and live account restriction establish the relevant failure; they do not prove this is the only unhealthy provider or guarantee recovery after one edit.

## Supported resolution

1. Have Nodies enable `debug_traceBlockByHash` and `callTracer` for this Base credential/plan, or configure a Base provider that supports those exact calls and the required historical range. No purchase or account change was made.
2. Keep native ETH trace completeness checks. Removing the trace call or silently treating ordinary block transactions as complete would miss internal ETH transfers.
3. After entitlement changes, wait until the displayed provider cooldown expires for automatic retry. When intentionally changing endpoint/credentials, use the existing audited backend configuration UI; do not clear SQL cooldowns or skip checkpoints to work around the restriction.
4. Verify that the provider's last error clears, a complete scan succeeds, live progress resumes, and the existing admission prerequisites become satisfied. Actual production recovery remains pending the provider/configuration change.

## Scope and specification review

No business code changed. Existing timed-deposit specifications already require native callTracer, persistent cooldowns, historical scanning and credential redaction; no executable contract changed, so a new permanent provider-specific specification was unnecessary. Rust/Web suites were not rerun for this diagnosis-only documentation slice. Validation consists of live read-only capability probes, source trace review, task validation, whitespace checks and secret-exclusion checks on new artifacts.
