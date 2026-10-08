# 首页 Pencil 1:1 复刻与轮播后端支持

## Goal

依据当前 `mobile/pencil/hippo-mobile-uiux.pen` 选中的首页设计，完成手机首页上半部分的生产复刻，覆盖已登录和未登录状态的资产/登录提示卡，并把原有市场简报替换为由后台配置驱动的推广轮播。轮播必须通过公开 API 下发，支持内部目标跳转、自动轮播、手势/键盘操作和无数据降级。

## What I already know

* Pencil 当前已完成浅色/深色首页资产卡与访客提示条的设计迭代，相关选中设计记录在任务 `10-08-pencil-replace-market-brief` 和 `docs/superpowers/PROGRESS.md`。
* 轮播选中的 Pencil 帧为 `r8zcT`、`w2hy2C`、`MnV84`、`W4ql6`；访客资产区最近选中帧为 `ZGf2W`、`u1hhY`；登录态资产概览已收敛为紧凑右侧曲线和单行今日收益。
* 生产首页入口为 `mobile/src/views/HomeView.vue`，当前访客 Hero 缺少注册入口，登录态仍渲染已从 Pencil 移除的周期按钮和曲线背景辅助线，市场简报仍由实时 ticker 计算。
* 新闻已有公开列表/详情 API，后台已有新闻编辑、图片上传、审计日志和版本控制模式。
* Rust 全站用户路由挂在 `/api/v1`，后台路由挂在 `/admin/api/v1`；Web 后台使用 React + Semi Design，独立配置页通过路由级懒加载和权限边界接入。
* 工作区存在与本任务无关的未提交改动，实施时必须保留，不得回滚或混入本任务提交。

## Decisions

* 使用单例 JSON 配置表保存首页轮播，配置包含 `revision` 和最多 12 个 slide；后台保存使用期望版本号进行乐观并发校验，在同一 MySQL 事务内写入配置和管理员审计。
* 每个 slide 包含稳定 ID、排序、启用状态、标题、描述、图片 URL、语言、类型化目标类型和目标 ID。目标类型只允许 `news_detail`、`news_center`、`referrals`，不接受任意外链或脚本。
* 公开接口只返回启用且语言匹配的 slide；`news_detail` 必须确认目标新闻仍为已发布，失效目标被过滤。没有有效 slide 时返回空数组，移动端隐藏轮播，不用演示内容兜底。
* 复用现有管理员图片上传接口，轮播配置只保存经过 URL 安全校验的返回地址。
* 后台新增“首页轮播”独立页面，复用 `content.news` 读写权限；编辑支持新增、删除、排序、启停、标题/描述、图片上传、目标选择、保存原因和冲突提示。
* 移动端轮播使用现有 Lucide 图标和路由；支持触摸滑动、紧凑分页点列及原生 range 选页、键盘左右键、自动轮播。焦点、悬停、页面隐藏或 reduced-motion 时暂停，卸载时清理监听器和定时器。
* 访客卡保留主题图片，增加注册次按钮；访客卡不请求账户数据。登录态继续使用真实钱包/收益接口，但按 Pencil 结构移除周期栏与曲线横线，保留当前周期的真实收益曲线。

## Requirements

### Mobile

* 2026-10-08 追加：继续按 Pencil 修正顶部卡片，访客按钮还原为 78×30、6px 间距，收紧文案、描边、阴影及背景轨迹；登录资产标签、金额/收益间距和右侧曲线末端按稿修正，保留真实数据。
* 2026-10-08 追加：轮播不显示暂停/开始按钮；移除相关控制和失效文案，保留自动轮播、滑动、分页及焦点/隐藏/低动态暂停，分页按 8px 间距与 20px 轨道还原。
* 已登录首页上半部分与浅色/深色 Pencil 选稿保持结构一致：余额、单位、今日收益、紧凑右侧曲线和资金操作布局不跳动。
* 未登录首页上半部分显示主题访客资产提示条，包含登录主按钮和注册次按钮；不得显示余额、收益或资金操作的伪造值。
* 轮播位于快捷入口之后、行情列表之前；使用公开接口数据，展示左文案、右侧图片、分页指示。
* 轮播点击按目标类型跳转到新闻详情、新闻中心或邀请好友；失效/空数据隐藏整个轮播区域。
* 加载、网络错误、图片无效、单页/多页、触摸、键盘、自动轮播、主题切换和 reduced-motion 均有稳定行为。

### Backend

* 新增公开 `GET /api/v1/home/carousel`，响应只包含可公开字段和当前有效 slide。
* 新增管理员 `GET/PATCH /admin/api/v1/home/carousel`，PATCH 要求 `expected_revision` 和非空 `reason`，返回递增后的 revision。
* 新增不可变迁移，单例行具备 JSON 合法性、版本、更新时间和更新管理员关联；写入必须原子化。
* 校验标题/描述长度、slide 数量、排序唯一性、图片 URL 协议/长度、语言格式、目标类型/ID；拒绝未知字段、非法 ID、重复 slide ID、超过 12 页。
* 公开查询过滤未启用 slide、语言不匹配 slide 和未发布/不存在的新闻目标；查询失败返回明确服务错误，不静默生成演示内容。
* 更新 OpenAPI、权限映射、后端源码合同/路由测试和迁移测试。

### Admin Web

* 导航新增内容运营下的“首页轮播”入口和权限映射。
* 独立页面可加载当前 revision，编辑 slide 表单，使用现有图片上传组件，拖拽/按钮调整顺序，选择类型化目标，删除/启用/禁用 slide。
* 保存前要求操作原因；保存成功刷新 revision；409 冲突保留当前草稿并提示重新加载。
* 页面在桌面和窄屏下无页面级横向溢出，控件保持可访问标签。

## Acceptance Criteria

* [x] 320、360、390、448px 下浅色/深色登录与未登录首页上半部分稳定状态无横向溢出或金额裁切；异步空配置仍按要求移除轮播。
* [x] 未登录状态不请求或展示私有钱包/收益数据，并提供登录与注册入口。
* [x] 登录态不再展示已从 Pencil 移除的周期按钮和曲线横线，真实收益曲线仍可用。
* [x] 有效轮播由后端返回，空配置时首页隐藏轮播，不出现硬编码演示卡。
* [x] 轮播目标只能跳转到已声明的内部路由；未发布新闻目标不会从公开接口泄漏。
* [x] 轮播支持触摸、键盘、分页、自动轮播和 reduced-motion/焦点/悬停/页面隐藏暂停。
* [x] 后台可以上传图片、编辑/排序/启停/删除 slide，并通过 revision 防止覆盖他人修改；审计包含前后配置快照和原因，不含认证秘密。
* [x] Rust 路由、迁移、OpenAPI、Web API/UI、Mobile 适配器/页面测试及类型检查通过。
* [x] 完成移动端窄屏/明暗主题和后台桌面/窄屏浏览器检查，更新 `docs/superpowers/PROGRESS.md`。

## Verification and Remaining Boundaries

* 详细证据见 `research/delivery.md`。此前 Rust 511/511，架构 11/11，文档 1/1，OpenAPI/实库路由 14/14；Web 106 文件 999/999，生产策略 15/15，覆盖率门禁 23/23。本轮 Mobile release gate 800/800、类型、双构建、产物与预算全部通过，未重新执行无改动的后端和后台门禁。
* 浏览器使用真实页面组件配合隔离响应，并非生产 API 端到端验收；实库测试为本地隔离 MySQL，不涉及生产库。上传检查验证真实上传组件的一次请求及返回 URL 回填，不验证生产对象存储。
* 本轮重新对照 Pencil 四种顶部卡片，标准访客卡和登录资产区均为 128px，登录/注册按钮为 78×30px、间距 6px，分页轨道为 20px、上间距 8px；移除显式暂停/开始按钮，保留环境暂停和自动播放。图表末端为独立 8px 圆点，正向线色与财务文字色分离，避免全局 `.positive !important` 覆盖。108px 轮播及 76px 图片保持不变。正式推广图片由后台配置，不把测试图片作为默认内容。
* 真实收益图不伪造 Pencil 示例曲线；只有两个真实采样点时为直线。长金额完整换行，英文访客文案允许卡片略增高。
* Web 包体检查失败：总 JavaScript `2626539 > 2600000` bytes；历史基线也超限，本次增加的编辑页仍计入总量，不能把整项门禁计作通过。未提高预算、未展开无关打包重构。
* 发布前应用迁移 0146、配置轮播及图片上传权限，并解决后台包体预算。任务保留进行中，等待最终视觉确认及范围内提交授权；未部署、推送或归档其他任务。

## Definition of Done

* Pencil 现有选稿对应的生产首页视觉和交互完成。
* Rust API、MySQL 迁移、管理员编辑页和 Mobile 轮播完整接线。
* 相关测试、类型检查、构建和 `git diff --check` 通过。
* 任务 PRD、进度记录和必要的规范更新完成。

## Out of Scope

* 不修改 PC 首页，不新增独立活动 CMS，不实现外部链接或脚本目标。
* 不改动钱包、今日收益、行情 WebSocket、登录认证的业务口径。
* 不回滚或整理其他任务的现有未提交改动。
* 不部署、不推送远程；提交前仅纳入本任务明确修改的文件。

## Technical Notes

* Mobile 入口：`mobile/src/views/HomeView.vue`、`mobile/src/api/news.ts`、`mobile/src/styles/prototype-parity.css`、`mobile/tests/pencil-selected-home-layout.test.ts`。
* Rust 路由装配：`src/lib.rs`、`src/modules/admin/routes.rs`、`src/modules/news/routes.rs`。
* Admin 入口：`web/src/admin/routes.tsx`、`web/src/admin/navigation.tsx`、`web/src/admin/access.tsx`、`web/src/shared/AdminImageUpload.tsx`。
* 相关规范：`.trellis/spec/mobile/index.md`、`.trellis/spec/mobile/backend-integration.md`、`.trellis/spec/backend/index.md`、`.trellis/spec/backend/public-news-contract.md`、`.trellis/spec/backend/database-guidelines.md`、`.trellis/spec/backend/quality-guidelines.md`。
