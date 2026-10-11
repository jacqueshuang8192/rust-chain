# Delivery

- Administrator user actions expose a permission-guarded funds-record SideSheet.
  Agent user management links to the portfolio funds tab, with the same component.
- Wallet changes show immutable per-bucket snapshots; funds view shows signed
  ledger amounts with business references. These are two views of user ledger
  evidence, not inferred net income or platform-only accounting journal entries.
- Spot/margin account, asset, change/balance type and business reference filters
  use bounded server pagination. Money stays decimal text and row identity
  includes account type. Display formatting retains source precision in tooltips.
- Agent API repeats token-derived subtree predicates in rows and COUNT after
  membership/active-ancestry checks. Admin reuses wallet.ledger.read and the
  existing internal-user policy. No wallet creation or financial writes.
- Real isolated MySQL at localhost13316: two new API cases passed, covering wrong
  auth, invalid filters, parent/child allow, sibling/other/ancestor deny, disabled
  ancestry, exact money, paging/count and unchanged wallet/ledger counts. Compared
  Admin/Agent rows for both accounts. OpenAPI schema checks include the new route.
- Backend combined library523 + architecture11 + documentation1 + OpenAPI14,
  all-target compilation and strict Clippy passed. Optional library DB branches
  may internally skip; the new integration tests did not skip.
- Web all111 files/1061 tests passed. Final affected-file regression95/95,
  typecheck, lint, build, policy15 and coverage23 passed.
- Existing release blocker remains: final total JS 2648188 bytes exceeds2600000.
  The earlier recorded baseline was already over budget. No threshold was raised
  and no unrelated bundle refactor was introduced.
- Ego TaskSpace16 uses real pages with in-memory auth storage and GET-only mock
  APIs, never production user data. Agent snapshots and exact-reference filtering
  passed at1728/1280/390px without document overflow. Admin SideSheet opens with
  the fixed target user, changes account_type to margin, stays within390px and
  closes on Escape. No unhandled API fixture requests or framework overlay;
  all intercepted API requests were GET. Test scripts/screenshots stay under /tmp.
- No push, deployment, private credentials or production balance changes.
- The isolated test MySQL instance on13316 was shut down, without touching the
  pre-existing MySQL service. TaskSpace16 was closed once. Preview5199 remains.
