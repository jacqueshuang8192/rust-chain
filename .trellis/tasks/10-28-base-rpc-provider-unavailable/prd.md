# Diagnose Base scanner provider unavailability

## Request
Investigate the user's Base deposit-scanner warning at 2026-10-28T10:29:26.715Z using the supplied Nodies RPC endpoint. Keep its API key out of repository files and diagnostic output.

## Scope and approach
1. Inspect the scanner's actual RPC methods, authentication, candidate selection and durable cooldown behavior.
2. Make a small number of paced, read-only requests to the user-supplied endpoint; record only sanitized status, errors and chain metadata.
3. Distinguish observed RPC failures from unverified production configuration/cooldown state. Provide the concrete supported configuration or minimal fix supported by evidence.
4. Verify any change, record findings and update PROGRESS.md. Preserve existing unrelated working-tree changes; do not alter production data, cursors or credentials.

## Acceptance
- [x] Establish current RPC connectivity and Base chain identity.
- [x] Check relevant block/log/trace capabilities when initial connectivity permits.
- [x] Explain how observed behavior relates to the aggregate warning without inventing its original cause.
- [x] Record evidence, limitations and actionable next steps without secrets.

## Outcome
The current credential's plan rejects `debug_traceBlockByHash` with JSON-RPC -32601. The user confirmed the same method in the production provider's last_error. Basic chain/block/sample-token reads work. Diagnosis is complete; operational recovery requires provider entitlement or endpoint changes and has not been performed. See `research/diagnosis.md` and sanitized `research/rpc-probe.json`.

## Plan
Diagnostic work first; implementation only if evidence establishes an in-scope code defect. Production provider/key last_error and cooldown_until may be required to identify the historical failure.
