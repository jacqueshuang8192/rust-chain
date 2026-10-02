# Administrator Active Trading Delivery

## Scope and implementation

Continued the existing matching task and uncommitted initial UI implementation.
Preserved unrelated dirty work, including margin user-history pagination.

- `/admin/seconds-contract/orders`: defaults to opened orders, with all-order
  history, user/email filters, paging, refresh and existing detail actions.
- `/admin/margin/positions`: defaults to opened **filled** positions. The new
  optional `filled_only` boolean filters both rows and total by non-null entry
  price. Omitted/false preserves the original API behavior, including pending
  limit orders. All records distinguish pending, held and closed states.
- Shared resource `fixedFilters` survive editable filter reset. Scope switching
  remounts the list, resets pagination and invalidates old request ownership.
- The shared seconds timer keeps agent behavior, runs locally once per second,
  resynchronizes on focus/visibility, and says awaiting settlement at expiry.
  It does not change order status or poll per row.
- State and countdown precede secondary identifiers in the seconds table.
  Existing permission guards, Decimal text, detail actions and readonly queries
  remain authoritative. There is no new trade mutation or database migration.

## Verification

- Initial focused frontend tests: 6 files, 110/110 passed.
- Final trading/timer tests: 2 files, 6/6 passed. Reset/refresh retain fixed
  filters, tabs reset paging, pending rows are not labeled held, timers clean up,
  and rendered table content belongs to the active accessible tabpanel.
- Initial full suite: 991/992. The one task-local test retained a timer DOM
  reference across a list reload; fixed by querying the newly rendered timer.
- Final full suite: 105 files, 992/992 passed after the assertion correction.
- `npm --prefix web run typecheck`, `lint`, `build`: passed after final UI edit.
- Production-policy 15/15 and existing coverage gate 23/23 passed.
- `cargo check --all-targets --all-features`: passed.
- `cargo test --lib --test backend_architecture --quiet`: 507/507 and 11/11.
- `cargo clippy --all-targets --all-features -- -D warnings`: exit 0. Cargo
  emitted an incremental-cache file deletion warning, not a code lint finding.
- `cargo fmt --all -- --check`: passed.
- `task.py validate` and `git diff --check`: passed.
- Dedicated MySQL 9.3, port 13318, disposable schema `admin_active_trading`:
  `DATABASE_URL=mysql://root@127.0.0.1:13318/admin_active_trading cargo test --test margin_routes admin_margin_ -- --nocapture`
  executed 12/12 successfully. The added route test checks active/all/false/
  historical filters, stable page IDs, total after page exhaustion, invalid
  boolean rejection, exact string entry price and unchanged record count.
- Test MySQL shut down using its private socket. Existing port 3306 untouched.

## Browser evidence

Ego TaskSpace 6 used the existing local Vite port 5198 with `browser-mock.js`.
All API responses were intercepted in the tab, including URLs configured with
the application API origin. No production requests or financial writes were
performed. The mock role had only the two existing read permissions.

- Real components showed active countdown, all-order history, current held
  positions, and all-margin held/pending/closed labels.
- Countdown changed between snapshots without a row request for each tick;
  Vite StrictMode's initial duplicate GETs are not polling.
- Page horizontal overflow was zero at 1728, 390 and 320 CSS pixels.
- Desktop seconds and mobile margin screenshots were visually inspected.
  Final seconds status/countdown were visible without horizontal table scrolling.
- TaskSpace closed. Existing Vite process left running.

## Final continuation verification

- `npm --prefix web run typecheck`: passed.
- `npm --prefix web run lint`: passed.
- Focused trading regressions: 2 files, 6/6 passed.
- `python3 .trellis/scripts/task.py validate .trellis/tasks/09-28-admin-active-trading`: passed.
- `git diff --check`: passed.
- No production requests, financial writes, migration, commit, push or deployment performed in this continuation.

These are local mocked browser tests plus a separate real MySQL API regression,
not a production or MySQL 8.4 end-to-end certification.

## Remaining release boundary

The existing bundle budget remains failing: JavaScript raw size 2,614,909 bytes
exceeds 2,600,000. The prior progress record already reported a raw/gzip overage;
this task does not raise budgets or refactor unrelated bundle ownership. Full
release readiness must not be claimed. No commit, push or deployment performed.
