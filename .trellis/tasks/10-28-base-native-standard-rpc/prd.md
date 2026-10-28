# Base native deposits through standard JSON-RPC

## Goal and authorization
The user requests removing the `debug_traceBlockByHash` dependency after confirming that their Base RPC plan rejects it. The preceding discussion explicitly distinguished direct ETH transfers from internal contract transfers. Implement direct-transfer scanning for Base with ordinary block and receipt methods; do not replace one restricted trace API with another.

## Scope
- Base native ETH: hashes from `eth_getBlockByNumber(height,false)`, `eth_getTransactionByHash` batches of at most 25, then `eth_getTransactionReceipt` only for positive-value transfers to watched addresses. The supplied plan also rejects fullTransactions=true and eth_getBlockReceipts, so neither is used.
- Credit only successful top-level transfers. Internal calls, contract creation destinations, zero-value transfers and self-transfers are outside this direct-transfer mode.
- Verify block/transaction/receipt identity, index, sender, recipient and receipt status. Missing/malformed/mismatched evidence fails the block without advancing the cursor.
- Preserve native event index `0x80000000` per transaction so old root-call receipts replay under the same existing database uniqueness key. ERC20 event indexes and Ethereum trace scanning remain unchanged.
- Keep persistent cooldowns, network leases/revisions, historical replay, confirmations and transactional accounting. Native Base rounds cap the existing adaptive batch at four blocks. No migration, production data edits or cursor reset.
- Update Base operator documentation/help to disclose direct-only ETH support. Keep other network contracts intact.

## Plan
1. Implement the Base-only path and focused RPC fixtures, including legacy trace-root identity compatibility.
2. Verify ordinary full-block/receipt methods on the supplied endpoint with a small number of read-only probes; never save credentials.
3. Update backend spec and operator help/docs, run focused backend/frontend checks and required gates, and record results/limitations.

## Acceptance
- [x] Base native and mixed native/ERC20 scans never call debug/trace methods.
- [x] Successful direct transfers preserve exact amount, confirmations, block time and legacy event identity.
- [x] Failed, zero, self, contract-creation, unwatched and internal-only transfers do not produce native receipts.
- [x] Missing/invalid/mismatched transactions or receipts reject the block.
- [x] Existing Ethereum internal-transfer regression still passes.
- [x] Block-hash lists, 25-transaction batches and receipts work on the supplied Base RPC; only sanitized evidence is stored. Record observed 100-call batch throttling and the sustained-throughput limitation.
- [x] Help/spec/docs describe the changed Base boundary and retained progress/cooldown behavior.
- [x] Validation and progress are recorded, and unrelated uncommitted work is preserved.

## Limitations
Changing Base to direct-only scanning intentionally excludes internal ETH payments, including historical ones not yet observed. Already-stored chain evidence is retained. Production recovery is unverified until the updated server is deployed and its persistent cooldown expires. No deployment or provider subscription change is part of this task.

## References
- Prior live diagnosis: `.trellis/tasks/10-28-base-rpc-provider-unavailable/research/diagnosis.md`.
- Existing adapter and settlement uniqueness: `scanners/evm.rs`, `scanners.rs`, `settlement.rs`.
- Ordinary receipt identity/status contract will be verified against provider responses and primary documentation.
