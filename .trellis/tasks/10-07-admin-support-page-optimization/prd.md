# 后台在线客服页面优化

## Goal

提升 `admin/support` 管理员处理客服会话的效率和可扫描性，保留现有接口、权限和消息幂等行为，不改变代理客服页面的业务口径。

## What I already know

- 管理员页面复用 `web/src/support/OnlineSupportWorkbench.tsx`，通过 `scope="admin"` 接入全局客服队列。
- 当前页面已经支持状态、未分配、当前页关键词、分页、自动同步、历史消息分页、标记已读、关闭/重新打开和失败重试。
- `web/src/api/support.ts` 已支持服务端 `unread_only`、分页和管理员接口前缀，但工作台没有提供未读筛选入口。
- 现有样式已有桌面双栏和窄屏断点，但队列和对话区缺少更明确的工作台层级和固定高度约束。
- 当前工作区有无关的未跟踪 `.vite/`，不能删除或提交。

## Assumptions

- 本轮只优化管理员 `/admin/support` 的呈现和交互；代理 `/agent/support` 保持现有行为。
- 不新增后端接口，不修改客服权限、归属规则、消息状态机和发送协议。
- 搜索仍保持“当前页关键词”语义，避免把本地筛选误认为服务端全量搜索。

## Requirements

- 管理员队列增加服务端“仅未读”筛选，并与状态、未分配筛选、分页和自动同步正确联动。
- 队列顶部信息更容易扫描：明确当前服务端总量、当前页结果、未读/未分配提示和同步状态。
- 当前会话在桌面端形成稳定的消息处理面板：标题、联系方式、归属、状态、操作和消息区分组清晰。
- 消息区和回复区在桌面端保持可用高度，页面级滚动减少；窄屏下不产生横向溢出，操作按钮和输入区仍可访问。
- 保留加载、空数据、错误、历史消息加载失败、发送失败重试和只读权限状态。
- 不影响代理端现有筛选控件和行为。

## Acceptance Criteria

- [x] 管理员可以切换“仅未读”，请求带 `unread_only=true`，并在切换状态/归属/分页时保持筛选一致。
- [x] 仅未读开启时，页面只展示 `staff_unread_count > 0` 的会话；刷新和轮询不会丢失筛选。
- [x] 管理员工作台在桌面宽度下具备清晰的队列/对话双栏层级，消息历史和回复区不被页面内容挤压。
- [x] 390px/840px 等窄屏宽度下客服页面无页面级横向溢出，搜索、筛选、会话操作和回复按钮可用。
- [x] Web 定向测试、类型检查、lint、构建和相关 Admin 质量门禁通过；包体 budget 在本次改动前的 `HEAD` 基线同样超限，详见进度记录。

## Definition of Done

- 新增或更新工作台回归测试，覆盖未读筛选和管理员专属呈现。
- 完成 `npm --prefix web run typecheck`、`lint`、定向测试和至少一次生产构建。
- 更新 `docs/superpowers/PROGRESS.md`。
- 完成桌面和窄屏浏览器检查；浏览器使用本地模拟数据时明确记录。

## Out of Scope

- 不新增客服分配/改派能力。
- 不改 Rust 客服接口、数据库结构、WebSocket 推送或消息状态机。
- 不新增快捷回复、附件、富文本或客服统计报表。
- 不重构代理客服页面。

## Technical Notes

- 主要文件：`web/src/support/OnlineSupportWorkbench.tsx`、`web/src/support/OnlineSupportWorkbench.test.tsx`、`web/src/styles.css`。
- 参考规范：`.trellis/spec/admin/index.md`、`.trellis/spec/admin/ui-system.md`、`.trellis/spec/admin/chinese-presentation.md`。
- 接口字段：`SupportConversationFilters.unread_only` 已存在，服务端列表响应提供 `total`。
