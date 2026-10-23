# 充值配置与代理充提记录交付

## 已实现

- 链上配置改为网络概览、监听币种列表、收款地址列表。节点、币种、地址采用独立编辑面板；取消、继续、放弃草稿均有明确入口，不再因任意字段修改而锁死整页。
- Ethereum 可分别添加 ETH、USDT、USDC 并继续添加更多已创建的平台资产。原生币身份/精度预填；代币必须输入已核对的合约/mint 和链上精度。已配置身份保持只读、可复制，说明不能原地修改的原因，允许启停。
- 全配置响应新增最小 `platform_assets` 目录，只含 symbol/status/deposit_enabled/precision_scale；网络读取角色无需额外资产管理权限。仅地址角色继续调用独立地址接口；只读角色看得到权限说明，不出现写入入口。
- 代理新增充值记录、提现记录及团队用户行入口，含分页、用户/分支/币种/网络/状态/时间筛选、充值来源/订单号及明细。充值合并快速充值与链上订单，原人工充值申请保持独立。
- 新接口 `GET /agent/api/v1/funding/{kind}` 从令牌派生代理子树，行、计数、所有 UNION 分支均重复限制归属；传入的 agent_id 只能缩小范围。详情使用已经授权的列表行。
- 金额以精确字符串传输，应付量、实收量、净入账量、手续费、提现总冻结量、法币报价分别标注。链上冲正/核对状态和到期状态按原合同展示，提现待审使用真实 `pending_review`。查询不变更订单、钱包、账本、分录或事件。

## 配置页问题原因与处理

旧编辑器将节点、资产、地址和网络选择共用一个 dirty 状态，输入后其他操作被禁用，且不易找到恢复方式。现在编辑器固定打开时的网络、记录和 revision，失败保留草稿，409 不覆盖旧版本；离开未保存编辑时显式选择继续或放弃。原生币不显示无意义的禁用合约框，已有身份以只读文本展示。

320px 验收另发现 Semi Card 的标题与按钮挤在同一行。已在本页窄屏样式中将标题和操作分行，并显式关闭标题省略，未改共享组件。

## 验证

- 最终 `cargo fmt --all -- --check`、`cargo check --all-targets --all-features` 通过；生产 `cargo clippy --lib --bins --all-features -- -D warnings` 通过。
- 隔离 MySQL：新增 funding HTTP 集成 2/2 通过，包含 root/child/grandchild、兄弟/外树/unassigned、用户邀请继承、用户转移、停用祖先、行/总数/分页、来源/状态、超 JS 安全整数的精确币量、敏感字段排除与全局金融快照无变化；无 DB 测试确保认证与非法筛选前置。最终移除临时浏览器夹具后再次 2/2 通过。
- 充值配置真实 HTTP 1/1 通过，验证最小目录与仅 `wallet.networks.read` 角色。OpenAPI 20/20、架构 11/11、后端文档 1/1 通过。
- Web 最终 typecheck、lint、build 通过。最终完整 124 文件、1164/1164 通过；只读权限和充提菜单测试定向 25/25 通过。
- production-policy 15/15、coverage 23/23 和原覆盖率门槛通过；没有放宽门槛。
- Ego 浏览器使用真实本地 API 与隔离 MySQL，无浏览器接口替身。实际连续保存 ETH、USDT、USDC，确认仍可选择 DAI；验证继续/放弃草稿、网络切换、节点/地址面板，以及代理充值/提现筛选、详情、用户直达。
- 桌面 1728px 和 390/320px 已检查截图。配置卡片、编辑器可用；代理表格内部横向滚动，页面无横向溢出。浏览器工具每轮视口会恢复，尺寸检查和截图在同一轮执行。

## 保留的门禁与边界

- Web 全 JS 包体最终 **2,749,426 bytes**，高于既有 **2,600,000** 预算。上轮记录基线 **2,731,976**，本轮增加 **17,450**；未修改预算或无关依赖。
- 扩大全代理实库回归为 **50 通过、3 失败、1 临时浏览器夹具忽略**。失败分别为既有 dashboard 邀请码夹具违反 `uk_invite_codes_agent_owner`、旧邀请码测试期待 usage_limit 25 而当前固定码返回已有 10、token lifecycle 测试缺少显式 `TOKEN_LIFECYCLE_REDIS_URL`。这些业务/测试代码本轮未改；不将扩大回归报告为全部通过。临时 ignored 浏览器夹具已删除，未纳入交付。
- 本轮无需新增迁移；运行环境仍需具备上轮迁移 0160。前后端应配套发布，因为全配置响应现在要求 platform_assets。
- 未连接正式链节点或执行实链转账；测试合约与地址仅用于隔离验收。实际监听仍需真实节点、收款地址、合约和 decimals，依照原五网络能力限制逐链验收。
- 不涉及私钥、提现审批/广播和生产部署；30 分钟精确组合占用、不加尾数、歧义人工核对规则保持原合同。

## 本地证据与清理

日志：`/tmp/funding-db-final.log`、`/tmp/funding-settings-db.log`、`/tmp/funding-backend-guards.log`、`/tmp/funding-clippy.log`、`/tmp/funding-check-final.log`、`/tmp/funding-tests-final.log`、`/tmp/funding-web-coverage.log`、`/tmp/funding-build-final.log`、`/tmp/funding-budget-final.log`、`/tmp/funding-agent-all.log`。

截图：`/tmp/funding-config-desktop-final.png`、`/tmp/funding-config-header-320.png`、`/tmp/funding-address-390.png`、`/tmp/funding-scanner-320.png`、`/tmp/funding-deposit-detail-desktop.png`、`/tmp/funding-withdrawal-390.png`、`/tmp/funding-withdrawal-detail-390.png`。均已打开检查。

本任务 Ego Space 6 已结束；Vite 3037、API 18087 和隔离 MySQL 13368 已关闭；临时浏览器夹具已删除。无关诊断任务和 `.vite/` 保留，不纳入提交。本轮只做本地提交及任务归档，不推送远程。
