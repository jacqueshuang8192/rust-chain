# Agent list single-panel grid width

## Goal

Fix the screenshot-reported agent list that occupies only the left third of the
published Admin workbench, while preserving the existing tab-based workflow.

## Confirmed evidence

- User screenshot shows the published list/create tabs and a narrow inner panel.
- HEAD uses `.admin-action-workbench-grid` with two tracks (`0.7fr` / `1.3fr`)
  while conditionally rendering just one panel, leaving the second track empty.
- Earlier full-width work was validated against an uncommitted redesigned page;
  it did not address the actual published template and was not pushed.

## Requirements

- Scope single-column layout and 100% outer width to agent management only.
- Keep list/create tabs, API behavior, permissions, pagination and table scroll.
- Preserve the unrelated local redesign; deliver only the minimal class/CSS fix.
- Verify the actual published component plus the patch in a real browser.

## Acceptance criteria

- [x] Reproduce the two-track gap against HEAD.
- [x] Panel and workbench grid have equal width at 2942/1920/1728/1280/390/320.
- [x] No page-level horizontal overflow; list/create tabs and inner scroll work.
- [x] Check populated and empty data, scoped CSS, Web quality checks and build.
- [x] Update progress and record precise staged scope so the fix reaches both remotes.

## Plan

1. Inspect screenshot against shipped JSX and CSS (complete).
2. Add a page class to the committed template and scoped full-width/single-track CSS.
3. Use a temporary HEAD-derived real-component fixture for visual verification.
4. Stage only the reviewed patch, document results and deliver under existing authorization.

## Out of scope

The pending six-column agent redesign, other Admin pages, APIs, permissions and
production deployment. The Base upgrade repair continues as its own tracked task.
