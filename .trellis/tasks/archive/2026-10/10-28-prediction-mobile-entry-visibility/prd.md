# 竞猜手机端入口显示开关

## Goal
在后台「竞猜配置」提供手机端显示开关，关闭后手机端不展示竞猜入口。

## Requirements
- 默认开启，保持现有行为；开关独立于 Polymarket 同步。
- 使用现有全局策略保存流程，包含原因、revision 并发保护和审计。
- 公共 `/api/v1/prediction/config` 返回 `mobile_entry_enabled`，无登录也可读取。
- 手机端产品中心按配置显示竞猜行，隐藏时布局自然收起。进入页面、重新回到前台时读取最新配置；加载期间及读取失败不展示未经确认的入口。
- 旧后台请求省略新字段时保留已有值；旧后端尚未返回该字段时兼容为开启。
- 仅隐藏入口；既有竞猜路由、订单、资金、结算、同步及 PC 页面不受影响。

## Acceptance Criteria
- [x] 默认开启；后台关闭、保存、重载后保持关闭，再开启可恢复。
- [x] 公共配置在空资产和非空资产两条分支都返回真实开关。
- [x] 后台并发冲突不覆盖开关，旧请求省略字段不重置开关，审计记录 before/after。
- [x] 手机端未完成加载不闪现入口，关闭/请求失败时隐藏，开启时可导航；新闻与帮助入口保持可用。
- [x] 后台、Mobile、Rust 对应测试、类型/格式/质量检查执行并记录。

## Implementation Plan
1. 新增不可变迁移，打通持久化、管理与公共 DTO、更新及审计。
2. 复用后台表单 Switch；扩展 Mobile 公共配置适配器，产品中心动态显示。
3. 执行回归、浏览器验证及规范/进度记录。

## Technical Notes
- 手机端唯一发现入口为 `mobile/src/views/ProductHubView.vue`。
- 配置读写在 `src/modules/prediction/{application,presentation,repository,infrastructure}.rs`。
- 后台表单在 `web/src/admin/actions/prediction/`。
- `fetchPredictionConfig()` 原返回资产数组，保留调用合同并增加完整配置读取接口。
- 不修改工作区已有代理管理界面改动。
- 需求范围已由用户明确授权，无阻塞问题；不额外扩展为停用竞猜功能或限制直接链接。

## Definition of Done
验收项通过，必要规范和 `docs/superpowers/PROGRESS.md` 更新；未执行的验证、迁移发布前置条件如实记录。

## Out of Scope
PC 入口、交易准入、直接链接拦截、已有订单处理、部署、推送。
