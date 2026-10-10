# Current Synchronization Diagnosis

## Evidence (October 11, 2026, Hong Kong Time)

| Sampling window | BTC WS median age | ETH WS median age | BTC lag growth |
|---|---:|---:|---:|
| 07:30:30-07:31:02 | 465,204ms | 465,309ms | 13,594ms in 27,794ms |
| 07:34:31-07:35:03 | 555,925ms | 555,974ms | 8,624ms in 28,550ms |

REST returned stale observations too (second sample ~551s old). Depth and K-line
times followed the same backlog. These are anonymous backend observations,
independent of the chart renderer. HTTP Date was consistent with the local clock.
The local Vite proxy ticker also returned stale external Bitget observations.

Both attempts to directly connect to Bitget failed from this machine; an
anonymous Bitget REST attempt also returned ECONNRESET. This run cannot establish
same-round exchange latency. Platform socket errors marked afterRequestedClose
occurred during diagnostic shutdown, not during receipt.

## Code / Delivery Findings

`git ls-remote origin refs/heads/main` returned
6530773d808c7cb4e977eee2397bbbf4fa3180ac. Local 8d07a73 contains the earlier
Redis/admission + independent archive/spot/margin/K-line consumer fix and has not
reached remote main. The remote revision still awaits MySQL ticker archive and
the synchronous ingestion path before reading/publishing subsequent frames.

The measurements are consistent with a serial-consumer backlog, and confirm
the fault is upstream of frontend rendering. They do not identify the exact
running production binary or prove MySQL's individual share of latency. No
production logs, slow-query metrics or Redis queue lengths were accessed.

Even local HEAD only checked inbound silence, so a continuously readable old
stream could remain connected indefinitely. The current task adds independent
source-envelope progress recovery, retaining the earlier pipeline decoupling.

## Operational Next Step

Build/deploy a revision containing both 8d07a73 and this task's fix, then repeat
the same bounded public probe. Do not mistake a local Vite restart for backend
deployment. Before enabling the durable pipeline, confirm persistent standalone
Redis/noeviction, capacity and four extra MySQL lock connections. Never delete
unprocessed lane queues to make the age indicator look healthy.

No push, deployment, server restart, private credentials or financial operations
were performed. A code-level regression pass is not production recovery proof.
