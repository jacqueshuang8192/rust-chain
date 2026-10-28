# Upgrade state analysis

## Evidence

- `scanners/evm.rs`: Base native returns before the only trace RPC invocation;
  Base token does not enter the native branch. `Node.network` is copied directly
  from the scanner configuration in `provider_runtime::scan_available`.
- Before stream isolation, `Node::rpc_result` persisted the exact error
  `RPC 方法 debug_traceBlockByHash 失败，请检查渠道权限和额度` at provider level.
- 0165 copies scanner errors/checkpoints to both streams and invalidates leases,
  but does not retire obsolete provider capability failures.
- `scan_available` excludes providers with a future shared cooldown before any
  RPC; exponential retry can reach 3,840 seconds for this error.
- `Gate::succeeded` clears the shared error only after an entire scan succeeds.
- Admin `ScannerProviderStatus` renders stored `last_error` even after its cooldown
  expires. A provider not selected, a disabled scanner, or another scan failure can
  therefore keep showing this old text without making a new trace request.

## Hypothesis to reproduce

An old Base trace failure persists across 0165 and blocks both new streams while
the shared provider remains cooling. Removing the trace call alone does not
remove this durable blocker. It is not yet proven to be the user's live cause;
the user has confirmed Base and Admin channel status, which reads this persisted
provider field, but no timestamp/process/database evidence is available.

## Proposed scope

Prefer a new immutable migration that retires only the exact obsolete Base trace
provider failure. Do not reset all channels or credentials, do not rewrite scan
checkpoints, and do not fake a successful scan. Test before/after request counts,
state preservation and unaffected quota/other-network failures using real MySQL.
