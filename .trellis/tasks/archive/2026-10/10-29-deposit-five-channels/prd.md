# Five deposit monitoring channels per supported network

## Goal
Provide at least five distinct provider channels for Ethereum, Base, Tron, Bitcoin, and Solana in the admin deposit configuration, using the existing primary/backup scheduler and preserving complete deposit evidence.

## Confirmed requirements
- User requests at least five channels per network for wallet deposit monitoring.
- Prior user choices apply: all five supported networks; primary/backup automatic failover and fewer duplicate requests; free accounts and administrator-owned API keys are acceptable.
- Prefer free public endpoints and free-account templates, with explicit authentication/setup/capability limits. Do not invent credentials or count testnet/duplicate endpoints as independent channels.
- Keep saved configurations, credentials, real cooldowns, cursors, and financial validation unchanged.
- No deployment or new paid subscriptions/account creation are authorized.

## Research and assumptions
- Extend account templates with explicit free / trial / paid / account-dependent labels; no subscription is opened or enabled automatically. An optional preference question about paid templates is pending; inclusion means only a selectable setup entry, never purchasing or claiming free capacity.
- Use public defaults only when previously qualified; newly probed dRPC Bitcoin and Tron public endpoints returned HTTP 403 and are not promoted to defaults.
- Verify documented methods and authentication for each candidate against the actual scanners.
- Public endpoint probes are low frequency and read-only; no historical user secrets.
- Account templates may be documented but cannot be authenticated/live-verified without administrator keys. Clearly distinguish them from probe-verified public channels.

## Acceptance criteria
- [x] Each supported network has at least five independently named compatible provider choices, with provenance and capability notes.
- [x] Admin can select/add templates, see signup/setup instructions, and retain independent secrets and drafts.
- [x] Missing credentials cannot be mistaken for a ready channel; legacy saved configurations continue to work.
- [x] Relevant backend/front-end regression, static/build checks and browser checks pass, with pre-existing limits documented.
- [x] Progress and delivery documents accurately distinguish verified samples, documented compatibility and production availability.

## Decision and scope
Extend the existing channel catalog/templates and their tests. Avoid a new scanner protocol unless the documented candidates require it and the change is necessary for five-channel support. Preserve the previously selected automatic primary/backup policy; do not fan out duplicate requests or weaken Ethereum internal transfer evidence.

## Out of scope
Production access/deployment, real deposits, buying plans, creating provider accounts, altering unrelated agent-management work, force-enabling five unconfigured channels, and changing settlement rules.

## Implementation plan
1. Inventory scanner requirements and current catalogs; verify official docs and public read-only probes.
2. Extend reusable provider templates and admin setup/selection for five networks.
3. Add targeted coverage; run required checks, update specs/progress, and save precise local commits.

## Research references
- research/providers.md (to be written)

## Confirmed implementation boundaries
- Changes are limited to admin provider templates, credential/endpoint validation and explanatory copy plus tests/docs. No scanner, schema, payment or runtime quota changes.
- Target selectable independent provider coverage: Ethereum Chainnodes/SolidRPC/Google Cloud/Alchemy/Chainstack; Base PublicNode/dRPC/Chainnodes/Alchemy/Chainstack; Tron TronGrid/Ankr/Alchemy/Chainstack/QuickNode; Bitcoin PublicNode/GetBlock/Ankr/Chainstack/dRPC; Solana PublicNode/Helius/Ankr/Chainstack/QuickNode.
- Dashboard-specific credentials use a full masked endpoint input with no fictitious host. Selecting such a template on an existing row must require a replacement endpoint; it cannot preserve a previous provider URL.
- TronGrid header mode must require an explicit key for its account template. Unchanged saved configurations retain current blank-secret behavior.

## Completion evidence
- Web full suite 1262/1262 passed before the final placeholder guard; final deposit suite 99/99 passed after it. Final exact-release build/type-check and lint pass. Policy 15/15 and coverage 23/23 pass.
- Existing JS budget remains exceeded: final exact version 2,804,007 / 2,600,000 bytes; no threshold changes.
- 1728/1280/390/320 browser checks passed; fixture API only, not provider credentials or real deposits.
- No scanner/backend/schema changes, purchases, production writes, deployment or remote push.
