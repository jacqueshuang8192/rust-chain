# Design review

## Scope decision
The invoked design-taste-frontend skill explicitly excludes dashboard/app screens. This is an existing shared application component, so the component exception was applied: keep the brand/type and working behavior, use applicable hierarchy, flat surfaces, contrast and screenshot review. No direction lottery/seed, stock images, generated imagery, marketing hero, video or motion quota was applied. No fabricated customer content was committed; browser sample conversations are local fixtures only.

## Baseline
Screenshot: /tmp/support-before-desktop.png.
The wide queue table hides the message preview behind horizontal scrolling; filters consume a large portion of the queue; bordered and shadowed message cards compete with the conversation. Source lint over old global CSS also reports unrelated shell patterns, which remain outside scope.

## First review
Screenshots: /tmp/support-after-desktop.png and /tmp/support-after-mobile.png.
Fixes: narrow the inbox and enlarge chat; restore the customer heading's intended size after a secondary-text selector matched it; remove the duplicate textarea counter; allow actions to wrap at narrow widths. Wait for sidebar transitions and loaded messages before screenshots; transient HMR/fixture loss is not an app authorization failure.

## Final review
Screenshots: /tmp/support-admin-{1728,768,390}.png and /tmp/support-agent-{1728,768,390}.png, all inspected.
The inbox exposes identity, preview, ownership, status, unread count and time in each row. Chat is the primary surface. Desktop has independent scrolling; narrow screens give chat its own view with a reachable return action. Long body text wraps and composer stays separate. No placeholder imagery or ornamental animation is needed for the operational task.

Authenticated live inspector probe/judge modules ran at 1440/768/390px; report: /tmp/support-design-inspection.json. It still reports unchanged global shell gradient/shadow/dividers, compact 11–12px metadata, and absent marketing pictures/hero. These are documented scope/app exceptions, not hidden measurements. No whole-page marketing gate pass or Awwwards score is claimed. Source lint passes; the three ignore comments specifically explain false positives for box-shadow: none.

## Interaction evidence
Both scopes, five widths: no document/inbox/chat horizontal overflow, no message/composer overlap; composer bottom remains inside viewport at the tested heights. Local fixture tests verify mobile draft return, failed-send retry identity, empty search, cached list retention after refresh failure, long-text wrapping and read-only mutation guards. Existing automated coverage validates server pagination, history cursors, filters, permissions, polling and retries. No production API write occurred.
