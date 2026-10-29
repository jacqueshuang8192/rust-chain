# 网络与收款配置页面全面优化

## Goal
重新组织后台网络与收款配置，让管理员更快查看网络可用性、处理渠道异常、配置币种和收款地址。

## Known facts
- 当前页面使用网络下拉选择，并依次堆叠RPC渠道、扫描进度、配置指引、币种、地址卡片。
- 五网络与免费渠道目录已经接入，保留主备自动切换。
- 当前工作区存在无关代理管理改动，必须保留。

## Proposed requirements
- 网络导航显示各网络概况，选中网络展示清晰的收款状态与待处理事项。
- 将收款配置（币种、地址）与监听运维（RPC、扫描进度）分区，减少长页滚动和重复说明。
- 精简渠道编辑与免费渠道选择的信息层级，保留完整凭据和费用限制提示。
- 保留权限隔离、保存确认、并发冲突草稿、错误提示和旧订单监听语义。
- 完成桌面及窄屏布局、加载/空态/错误/只读状态验证。

## Decision
用户于2026-10-30选择方案1：网络工作台。左侧网络导航，顶部状态与待办，右侧收款配置/RPC渠道/扫描与恢复；默认展示收款配置。

## Open questions
无阻塞问题。

## Acceptance criteria
- [x] 五网络切换清晰，状态不把未配置或过期数据标记为正常。
- [x] 币种、地址、渠道及恢复操作保留原请求契约和权限约束。
- [x] 窄屏不横溢，编辑/保存/取消/冲突流程可用。
- [x] 运行相关测试、类型检查、lint、构建及浏览器验证；记录既有门禁问题。

## Scope
本轮改页面布局与交互；不更改到账账务规则、不新增RPC服务商、不部署。

## Technical notes
- web/src/admin/deposits/DepositSettingsPage.tsx
- web/src/admin/deposits/RechargeWorkspace.tsx
- web/src/admin/deposits/MultiScannerForm.tsx
- web/src/admin/deposits/RechargeWorkspace.css
- .trellis/spec/admin/index.md
