# Implementation and verification map

- Base-only state table, legacy network row retained as configuration/revision.
- Independent loops prevent head-of-line waiting as well as cursor/error coupling.
- Three shared mechanisms remain intentional: network config fencing, finance lock,
  provider/key quota. Scope application RPC cooldowns by provider + stream only.
- Migration and isolated MySQL scenario passed. Initial scenario fixture used an
  invalid native symbol; corrected to ETH without changing production validation.
- Full wallet run initially passed 61/62: old Base snapshot test still wrote the
  legacy network cursor. Updated to seed stream rows and verify per-stream errors;
  focused rerun passed. No production fallback to stale legacy health was added.
- Web focused 52/52 and full 1215/1215 passed; Clippy and types passed.
- Final static/budget/browser checks and delivery notes follow.
