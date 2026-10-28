# Published agent list grid repair

## Root cause and scope

The published tab-based page conditionally renders a single panel inside the
shared two-track workbench grid. At 1728px, the panel was 468.648px inside a
1359px grid (34.48%). The screenshot matches this version. Earlier local
full-width validation used an uncommitted redesigned component, which was not
the published page. This repair adds only the agent page class and eight scoped
CSS lines; the unrelated six-column redesign remains local.

## Browser evidence

Ego used the exact HEAD component with the one-class patch, current shared CSS,
and a temporary local fixture with in-memory API responses. Final geometry:

| Viewport | Grid | Panel | Page / parent content |
|---|---|---|---|
| 2942 | 2573 | 2573 | 2679 / 2679 |
| 1920 | 1551 | 1551 | 1657 / 1657 |
| 1728 | 1359 | 1359 | 1465 / 1465 |
| 1280 | 967 | 967 | 1057 / 1057 |
| 390 | 237 | 237 | 311 / 311 |
| 320 | 167 | 167 | 241 / 241 |

No document-level horizontal overflow. Wait for responsive sidebar state before
measuring; immediate resize initially captured stale expanded-sidebar geometry.
320px create/list tabs retained 167px panel width. Table scrolling reached 350px
while document scrollLeft stayed zero. 1280px empty state also fills its grid.
Screenshots /tmp/agent-grid-fixed-1728.png and /tmp/agent-grid-fixed-320.png were
viewed. These use local API fixtures, not a production login or write.

## Exact release checks

Exported tracked HEAD web source to an isolated temporary directory, then applied
only the reviewed class and CSS patch. Reused installed dependencies through a
symlink; no uncommitted redesigned TSX or tests entered the release check.

- Typecheck and lint passed.
- Full Vitest: 1204/1205 passed. The existing TimestampText timezone assumption
  accepts UTC/China time and failed under Europe/Istanbul (15:00).
- Re-ran that file with TZ=UTC: 11/11 passed, with no source/test changes.
- Production-policy 15/15 and coverage gate passed (23 tests).
- Production build passed. Existing raw JS budget remains exceeded:
  2,781,813 / 2,600,000 bytes; no threshold relaxed.
- No extra source-mirroring test for this reversible CSS/class change; actual
  published-component browser geometry is the primary regression evidence.
- Temporary browser components/session/TaskSpace/Vite were cleaned up.

## Delivery boundaries

Stage the exported component (one scope class), shared CSS, only this spec appendix
and this task's progress entry. Leave all earlier AgentManagementPage, CSS, tests,
fixtures and spec edits in the working tree. Production deployment and remote CI
are not covered by local checks.
