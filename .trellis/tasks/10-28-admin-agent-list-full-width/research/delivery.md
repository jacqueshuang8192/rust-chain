# Agent list full-width correction

## Change

Added only `.exchange-page.agent-management-page { width: 100%; }` to the
existing uncommitted AgentManagementPage.css. The shared page cap remains
unchanged for other routes. Preserved the current page padding, table sizing,
resize handles, local pagination, filters and detail sheet behavior.

The original 1920px populated view had a 1560px page inside a wider content
pane. After correction, the page equals its parent's available client width,
and the Card equals page width minus its existing horizontal padding.

## Verification

- Targeted agent/helper/DataTable/ResizableTable tests: 4 files, 33/33 passed.
- Web full test suite (TZ=Asia/Hong_Kong): 126 files, 1212/1212 passed.
- typecheck, lint and production build passed.
- Production policy: 15/15 passed. Coverage: 23/23 passed, thresholds passed.
- Bundle budget remains an existing failure: JS 2,785,097 / 2,600,000 bytes,
  unchanged from the prior task's baseline. No budget was raised.
- Browser actual AdminLayout/AgentManagementPage with the existing local API
  fixture: widths 2560, 1920, 1728, 1280, 390 and 320, with no document overflow.
  Expanded and collapsed sidebar layouts both filled available content width.
  Stable measurements used reduced motion after observing sidebar transitions;
  a collapsed sidebar intentionally stays collapsed when the viewport widens.
- All measured page widths matched the parent client width; all list card widths
  matched page width minus padding. Tables filled the stage on wide screens
  and retained the 1020px internal scroll width at narrow sizes.
- Resize keyboard input changed the identity column from 240px to 256px;
  filtering AGT-003 yielded 1/23 and reset restored 23 rows. A 320px details
  sheet occupied exactly 320px; Escape closed it. Empty view at 1280px matched
  its 1072px content pane and had zero document overflow.
- Screenshots were inspected for wide, narrow, detail and empty states.
- git diff --check and task context validation passed.

## Scope and delivery

Only a page-local width rule plus spec/task/progress records were added this
turn. Existing uncommitted agent-management work is retained. No commits,
pushes, deployment or backend writes were performed in this task. Real backend
authentication/persistence were not exercised by the local browser fixture.
