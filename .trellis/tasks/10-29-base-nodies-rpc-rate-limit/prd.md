# Fix Base RPC burst pacing and adaptive transaction batches

## Goal

Reduce repeated Base `eth_getTransactionByHash` HTTP 429 by correcting durable request admission and learning a smaller transaction batch size without weakening deposit evidence, cooldown or checkpoint guarantees.

## Requirements

- Distinguish the current quota/rate failure from the retired trace-permission failure.
- Verify documented method costs, batching semantics, throughput limits, and available lower-request approaches.
- Inspect native/token scheduling, request accounting, API-key rotation, and retries without exposing credentials or accessing production data.
- Preserve unrelated working-tree changes and the existing deposit accounting, cursor, and lease guarantees.

## Acceptance Criteria

- [x] Findings cite official documentation and concrete local code paths.
- [x] Request amplification and relevant rate-limit behavior are explained.
- [x] Provide a feasible remedy with explicit limits and any required account information.
- [x] Record verification and remaining unknowns in the project progress log.

## Unknowns

- User confirmed Free Plan and sufficient remaining monthly quota. The console does not display numeric RPS. Shared-account consumption remains unknown.
- Whether this message is from a fresh scanner request or a retained channel status.

## Out of Scope

- Changing production configuration, upgrading a paid subscription, transferring funds, or probing the user's live key.
- Unrelated agent-management work.

## Validation

Use real isolated MySQL and mock RPC for deadline races, quota feedback, configuration lifecycle and migration replay, then run scanner regressions, architecture and Rust static checks. No live Nodies load test or production access is part of this verification.

## Findings and next decision

See `research/diagnosis.md`. User confirmed Free Plan, sufficient monthly quota and no visible numeric RPS. Implement feedback-based batch sizing rather than inventing an account RPS. Full-chain native lookups still have high sustained demand; the prior endpoint rejected full transaction blocks, so do not blindly enable that path. Nodies authentication guidance remains an operator check; no generic authentication behavior is changed here.

## Implementation scope confirmed by continued troubleshooting

- Fix the durable gate race: after sleeping, reacquire the network/provider/key locks and re-evaluate the latest next-request deadlines, cooldowns and lease before admitting a request. Do not accumulate expired reservations into bursts.
- Since numeric RPS is unknown, do not invent a per-second allowance. Persist a per-key Base transaction batch cap, initially 25. A batch HTTP 429 lowers it to at most half the rejected batch size (minimum one), while retaining Retry-After and shared cooldowns. Duplicate failures from the same batch size must not repeatedly halve the cap.
- Successful requests and process restarts retain the learned cap; explicit endpoint/key replacement resets it. Blank saves preserve it.
- Reduce native block work as the batch cap shrinks so slower batches do not repeatedly restart a four-block round. Preserve full-block evidence validation, cursor fencing and receipt identity.
- Do not change token parsing, enable restricted methods, infer incoming funds from balances, or promise that a low-rate plan can sustain full-chain native indexing.

## Implementation acceptance

- [x] Real isolated MySQL regression reproduces the old wait/deadline race and passes after the fix.
- [x] Mock HTTP 429 shrinks the persisted batch limit; subsequent node construction sends smaller batches and retains the limit after success.
- [x] Native/token sharing, cold keys, stale leases/configuration, blank saves, credential changes and migration replay retain their guarantees.
- [x] Closest scanner/database regressions, formatting, architecture, compilation and Clippy pass; actual Nodies throughput remains explicitly unverified.
