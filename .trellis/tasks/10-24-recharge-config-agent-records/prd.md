# 链上充值配置优化与代理资金记录

## Goal

优化链上充值配置交互，明确一个网络可以监听多个资产并支持继续添加；修复页面无法点击的问题；代理后台能查看所属下级用户的充值与提现记录。

## Known requirements

- Ethereum 可配置 ETH、USDT、USDC，后续添加更多明确链上资产；保留当前五网络及精确金额、30 分钟占用和人工核对规则。
- 用户反馈配置页有些地方完全点不了，需复现并从交互/权限/禁用状态排查。
- 代理只读查看下级用户的充值、提现，使用现有层级归属和权限边界。

## Assumptions

- 下级范围沿用现有代理团队用户可见范围，包含下级代理绑定用户；不同代理树严格隔离。
- 充值记录同时涵盖快速充值与链上订单及其状态；提现记录涵盖已申请和后续状态。
- 不预填未核对的合约、不自动启用生产配置、不涉及私钥或提现操作权限。

## Open Questions

- 暂无阻塞问题；先核对现有代理记录接口与页面点击问题。

## Acceptance Criteria

- [x] 同一网络多个资产以明确列表展示，可新增、查看、启停；必填/只读/不允许修改原因可见。
- [x] 编辑节点、资产或地址时无无提示的全页锁死；保存冲突、失败、未保存退出保留可靠处理。
- [x] 代理充值与提现记录有真实 API、分页、筛选及明细，不能读取上级/同级其他树用户的数据。
- [x] 金额精度、身份、权限、分页和点击回归测试通过，浏览器验证桌面/窄屏。
- [x] 更新规范、PROGRESS 与交付记录，明确既有门禁和实链验证边界。

## Out of Scope

改写链监听适配器、自动开通新合约、私钥、归集、提现审批/广播、生产部署、修复无关包体问题。

## Delivery

Implemented and verified; see [delivery record](research/delivery.md) for commands, browser evidence, inherited budget and unrelated full-suite limitations. No remote push, production deployment or live-chain transfer.
