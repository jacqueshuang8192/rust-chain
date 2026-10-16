# PWA 注册验证码与闪兑确认弹窗样式

## Goal
按用户要求修复注册发码按钮换行、增强可见性，并将闪兑 swap-review 弹窗与全局手机端样式统一。

## Requirements
- 发码按钮按文字宽度布局、保持单行和至少44px点击高度，适配明暗主题及发送中/倒计时。
- 闪兑确认使用全局主题变量、24px顶部圆角、448px画布上限、清晰明细及主次按钮；窄屏和横屏不溢出。
- 保持现有发码、协议确认、报价及交易提交逻辑。

## Acceptance Criteria
- [x] 320/390/448px及横屏布局检查通过，中英文和明暗主题可读。
- [x] mobile release:gate通过。
- [x] 更新PROGRESS.md。

## Scope
仅上述两个组件的展示样式，不修改API、金额计算、其他页面或依赖。

## Plan
1. 读取组件和全局主题规范，调整局部样式。
2. 执行发布门禁及本地浏览器布局验收。
3. 记录验证和交付。

## Decisions
用户两条请求已明确范围；使用项目现有主题和弹窗语言，无阻塞问题。

## Delivery
- Registration code action has content-based width and a mint fill; global focus/disabled styles remain authoritative.
- Swap review uses the global theme and body Teleport, bounded scrolling, summary rows and primary/secondary actions. Removed unused embedded-history CSS from SwapView; history is already a separate route.
- Final `npm --prefix mobile run release:gate`: exit 0, 850/850 tests, both builds and unchanged budgets passed.
- Browser: 64 registration cases (two locales/themes, four viewports, four states) and 8 swap cases (two themes, four viewports) passed. Final screenshots inspected; Escape and scroll restoration passed. Memory-only fixtures; no code-send or transaction submissions.
- Not committed, pushed, deployed, or tested on a physical installed PWA. Local Vite on 5186 retained.

## Follow-up: shared login-required redesign

User requests a redesign of login-required after reviewing Swap. Extend scope to
LoginRequiredState.vue and its existing visual contract checks. Replace the
striped alert-like treatment with a rounded theme-native card, soft icon tile,
clear title/description and a full-row login action. Preserve wallet compact
variants, localized descriptions and exact redirect query. Validate default,
wallet and history consumers in both themes/locales at mobile widths; rerun the
mobile release gate and record progress. Existing prior edits remain in scope.

### Follow-up delivery
- Shared login-required card redesigned; wallet compact variants preserved.
- Final release gate exit 0: 850 tests, both type checks, PWA/Tauri builds and all budgets pass.
- Browser 96 cases across six routes, four sizes, two themes and two locales pass; exact redirect including query verified. Light/dark screenshots inspected.
- Local commit was planned at this point but deferred during follow-up revisions; no push or deployment.

## Revision: centered login-required empty state

User rejected the card treatment and explicitly requests centering in the page,
using design-taste-frontend. Apply its single-component exception: keep global
brand/type, skip marketing direction draw, imagery floors and section motion.
The claim is simple: authentication enables the named operation; proof/action is
the existing real login route with the original redirect. Use an unboxed vertical
composition, readable heading/description, one flat pill CTA and the existing
Lucide lock. Center within remaining viewport, with natural scrolling on short
screens. Preserve compact wallet variants. No copy or auth logic changes.

Critique round 1 (previous screenshots): the bordered card clings to the header,
the full-width CTA overpowers the copy, and the horizontal icon/copy arrangement
reads as a notification. Replace these with viewport centering, a constrained
CTA and a single vertical axis. Component-only critique; marketing-page imagery
and creativity scores do not describe an app authentication empty state.

## Follow-up: neutral light-theme surfaces

User requests improving the pervasive green-grey #eef2f0 background, invoking
design-taste-frontend again. Scope is the Mobile/PWA shared light palette and
its duplicate transfer/contract/legal sheet tokens, not admin/PC or dark theme.
Keep existing brand typography, accent and semantic states; use white page
canvas, #f6f6f6 recessed controls, #ededed pressed/deeper surfaces, and neutral
borders. Preserve focus, disabled and selected contrast. Inspect registration,
swap, root pages and body-teleported sheets in real browser, recheck dark tokens,
run the full mobile release gate and update the tracked theme assertions.

### Bottom navigation follow-up
Selected ordinary bottom-nav items change icon and label to the semantic green;
remove their inherited green fill/gradient. Preserve routing, focus targets and
the existing always-raised Trade chooser. Verify actual computed background,
SVG color and label color after selecting each tab in light/dark themes.

## Final validation
- Centered login state replaces the superseded card design; prior 96-case browser matrix and redirect verification passed.
- Neutral light palette and ordinary green-text/icon active tabs passed the final 850-test mobile release gate, including both builds and all budgets.
- Four ordinary tabs in both themes were clicked in the real browser after a full reload: icon and label colors match, backgrounds are transparent, theme-color is correct and no document overflow occurs.
- New shared/teleported palette declarations are covered by existing source/contrast tests; not every recolored sheet was re-opened in this final browser pass. No real-device/installed-PWA claim.
- Source design lint passed. Full-page design inspection still reports existing desktop shell/marketing-rule violations, outside this component-only change.
