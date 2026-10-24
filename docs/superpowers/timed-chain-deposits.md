# 统一充值配置与验收

充值只保留“快速充值”和“链上充值”。快速充值沿用 GMPay；链上充值采用“网络＋链上资产＋地址＋精确币种数量”占用 30 分钟。不同金额可共用地址，同金额使用其他地址；该金额的地址全部占用时提示重试，不添加尾数、不换算法币。已付款组合也保留到原截止时间。

## 启用步骤

1. 应用尚未执行的迁移 `0158_timed_chain_deposits.sql`、`0159_timed_deposit_text_identity.sql` 和 `0160_independent_recharge_addresses.sql`。不要修改已应用文件。0160 将收款地址从旧永久库存中独立出来，自行生成编号，保留已有链上订单的地址引用。
2. 配置现有 `CREDENTIAL_ENCRYPTION_KEY`，要求 32 字节并持久保管。节点地址与 API 密钥加密存库；后台留空表示保留，不回显原值。
3. 打开“钱包资产 → 链上充值 → 网络与收款配置”，选择 Ethereum、Base、Tron、Bitcoin 或 Solana，点击“编辑监听配置”填写只读节点与确认数。
4. 在“监听币种”点击“新增监听币种”。例如 Ethereum 先添加原生 ETH（自动填 18 位精度），再分别选择 USDT、USDC，填写各自在 Ethereum 的真实主网合约和 decimals，后续仍可继续添加。币种选项来自平台资产；若未找到，先在资产管理创建后刷新本页。可显式选择下文主网预设填入合约和精度，也可手动填写；资产简称不代表链上身份。已有映射通过“编辑 / 启停”管理，合约与精度固定并显示不可修改原因。
5. 在“收款地址”点击“添加收款地址”，在独立面板粘贴地址，每行一个，单次最多 500 个。每网可先放 10 个，后续继续添加，没有 10 个硬上限。重复地址跳过，已停用地址不会因重复导入重新启用；任一地址无效则整批不保存。无需库存编号或二次纳入。Ethereum/Base 即使使用相同地址，也要分别添加到各自网络。
6. 检查平台资产的充值开关、最低充值数量、固定手续费和精度。数量必须高于手续费且满足链上及钱包精度，钱包最多 18 位小数。旧网络白名单、旧地址库存和地址组不再参与充值准入；“提现网络”只维护提现所需目录。
7. 开启“接受新充值订单”，等待最近追上区块的成功时间。节点未同步、报错或成功时间超过 120 秒时，不接受新订单。节点已配置时会持续监听，关闭接单开关、停用地址或资产都不影响已创建订单的到账核对。配置更新及新增地址会清除就绪状态，重新追上区块后才能接单。

编辑面板都有“取消编辑”；填写后关闭会提示是否放弃草稿。保存冲突保留草稿和原因，取消后刷新再重新编辑即可读取最新版本。网络选择不会再因为改动一个表单就被无提示锁死。

后台“订单与到账核对”查看订单和处理归属歧义，“入账流水”查看已入账记录；“快速充值”工作区集中维护 GMPay 配置和订单。用户端先选择链上币种与网络，再输入例如 100 USDT 或 50 ETH 的数量创建订单。

测试环境已取消旧永久地址兼容：旧分配 API、旧库存 CRUD 和旧观测到账 API 均退役。旧网关充值回执进入死信，不按旧地址归属自动加币；已有历史账务保留。提现业务仍使用原网关流程。

五网络配置边界：

| 网络 | 本批资产 | 所需节点能力 |
|---|---|---|
| Ethereum | ETH、明确配置的 ERC20 | 主网 chainId 1；完整区块、按 blockHash 的日志、decimals；原生 ETH 还要求 `debug_traceBlockByHash` 的 `callTracer` |
| Base | ETH、明确配置的 ERC20 | 主网 chainId 8453；同上，原生内部转账依赖 callTracer |
| Tron | 明确配置的 TRC20 | 主网 FullNode，`getnowblock`、`getblockbynum`、`gettransactioninfobyblocknum`、合约精度常量调用；原生 TRX 暂不开放 |
| Bitcoin | BTC | 已同步的主网 Bitcoin Core JSON-RPC，`getblockchaininfo`、`getblockhash`、`getblock` verbosity 2；使用可访问的历史区块，逐 vout 匹配，coinbase 不作为充值 |
| Solana | SOL、经典 SPL Token | mainnet-beta 的完整 finalized `jsonParsed` 区块、inner instructions、mint 元数据、slot/区块时间；Token2022 暂不开放 |

节点凭据可放在节点 URL 的受控鉴权路径中；API 密钥字段在 Tron 使用 `TRON-PRO-API-KEY`，其他网络使用 Bearer。Bitcoin Core 的用户密码鉴权可使用节点 URL 的 Basic Auth 信息，务必使用受信网络或 TLS 代理。所有地址只需要收款地址，系统不接收私钥。

## 主网节点与币种预设（2026-10-24）

在“编辑监听配置”选择“主网节点预设”，或在“新增监听币种”选择“主网币种预设”。选择只填充草稿，填写操作原因并确认保存后生效；接单状态不会自动改变，已有币种身份不能通过预设覆盖。列表缺少平台币种时，先在资产管理创建再刷新。

| 网络 | 节点预设 | 确认深度 | 币种预设 |
|---|---|---|---|
| Ethereum | Alchemy HTTP，替换 URL 中 `{API_KEY}` | 12 | ETH / 18、USDT / 6、USDC / 6 |
| Tron / TRC20 | `https://api.trongrid.io`，密钥填 API 密钥框 | 20 | USDT / 6 |
| Solana | 公共主网或 Helius（替换 URL 中 `{API_KEY}`） | 32 finalized slots | SOL / 9、USDT / 6、USDC / 6 |

Ethereum Alchemy 模板为 `https://eth-mainnet.g.alchemy.com/v2/{API_KEY}`；Helius 模板为 `https://mainnet.helius-rpc.com/?api-key={API_KEY}`。这两项的密钥直接放入 URL，下方 Bearer 密钥框无需重复填写。占位符未替换时不能保存。TronGrid 使用 `TRON-PRO-API-KEY` 请求头。密钥框留空保留已有密钥。

Solana 公共节点 `https://api.mainnet-beta.solana.com` 是真实主网，仅适合测试接入，持续完整区块扫描可能受限流影响。专用节点也需确认方法权限与额度；Ethereum 原生 ETH 监听必须支持 `debug_traceBlockByHash` / `callTracer`，Solana 需要 finalized 完整 jsonParsed 区块和 mint 查询。预设不是节点可用性或实际到账保证。

| 网络 / 币种 | 主网合约或 mint | 精度 |
|---|---|---|
| Ethereum USDT | `0xdac17f958d2ee523a2206206994597c13d831ec7` | 6 |
| Ethereum USDC | `0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48` | 6 |
| Tron USDT | `TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t` | 6 |
| Solana USDT | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` | 6 |
| Solana USDC | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` | 6 |

ETH 和 SOL 为原生币，无合约或 mint。收款地址仍需自行添加，预设不生成地址或私钥。

本轮验证与限制见[主网预设交付记录](../../.trellis/tasks/archive/2026-10/10-24-chain-recharge-presets/research/delivery.md)。

参考 [epusdt 固定版本](https://github.com/GMWalletApp/epusdt/blob/aed4a970a28d734c8a35499496604b868a24ef7f/src/model/dao/mdb_table_init.go)，并核对 [Tether](https://tether.to/en/supported-protocols/)、[Circle](https://developers.circle.com/stablecoins/usdc-contract-addresses)、[TronGrid](https://developers.tron.network/reference/select-network)、[Solana](https://solana.com/docs/references/clusters)、[Alchemy](https://www.alchemy.com/docs/reference/debug-traceblockbyhash) 和 [Helius](https://www.helius.dev/docs/api-reference/rpc/http/getblock)。本项目使用 HTTP 扫描，未照搬上游 Ethereum WebSocket 地址和较低确认数。

## 自动入账与人工核对

- 一笔已执行且确认达标的转账，只有在网络、合约/mint、收款地址、精确数量、链上付款时间都匹配唯一订单且无歧义时自动入账。
- 链上及时付款但扫描较晚，仍按历史付款窗口匹配。客户端倒计时和扫描时间都不决定资金归属。
- 30 分钟到期立即释放组合。此前有未解决的过期未付订单时，后续同组合到账进入核对；它可能是旧用户迟付，也可能是新用户正常付款。
- 错额、分笔、迟付和重复多付款分别保留证据，不自动合并。区块时间只有秒级时，下单同秒且先后无法证明的付款也需人工核对。
- 后台可按用户提供的完整充值订单号查找，读取对应内部编号。核对页面要求运营明确填写目标内部订单编号并读取验证；不会把候选订单自动当成付款归属。
- 人工入账仍不能绕过网络/资产/地址、确认数、钱包精度和单次入账约束。允许有依据地处理迟付、错额或下单同秒的款项；使用实际到账量扣除订单手续费，原预填金额保留。
- “排除证据”只是不入账，不退款、不转出资金，也不删除过期未付历史。没有充分证据消除迟付可能时，同组合后续付款仍需核对。
- 如需冲正，使用现有已入账充值流水的冲正流程；订单读取会反映冲正状态。

权限分离：`wallet.networks` 管节点/资产配置，`wallet.address_pool` 管共享地址，`wallet.deposits.read` 查订单/到账证据，`wallet.deposits.review` 执行人工核对。

## 代理查看下级充提记录

代理后台新增“充值记录”和“提现记录”，默认包含本级及下级代理归属用户。可按本级/下级、代理分支、用户、状态、币种、网络和创建时间筛选；充值还可按快速/链上及完整订单号查询。团队用户列表提供该用户的充值、提现记录入口。

充值记录包含待付款、过期、待核对、已入账和已冲正等状态。应付数量、实际收到数量、净入账数量与手续费分别展示；快速充值的法币报价在详情中单列。提现保留待审核、广播结果待核实、链上确认等真实状态。记录页只读，不能审批、广播或直接入账；原“我的充值申请”仍用于代理人工充值申请。

## 中断与恢复

正常重启保留持久游标；重复扫描不会重复加币。暂时性节点错误不推进游标，修复节点后会重试。停用地址/资产只影响新订单；历史监听身份保留。

已存在订单但游标缺失，或确认区块发生重组时，监听停止并关闭新订单准入。不要清空订单、删除证据或把游标直接跳到最新高度。本版本没有自动深度重组恢复入口：先关闭新订单接单并由维护人员暂停相应监听 worker、保存数据库与链证据、对账受影响已入账记录并通过原冲正流程处理，再由维护人员审查恢复高度与哈希并恢复游标。接单开关不停止扫描，不能将其当成维护暂停开关。恢复范围必须覆盖未处理历史；需要修正已存在证据时应单独实施受审计的数据修复。

## 本地验证与发布边界

本次重构使用隔离 MySQL，验证迁移重复执行、地址直接导入/去重/整批回滚、网络独立配置、并发占用、到期释放、幂等、历史付款归属、人工入账、手续费/钱包/流水/平台分录一致性、冲正，以及暂停新单后继续扫描和入账。真实 HTTP 测试覆盖金额类型、用户隔离、地址角色独立权限及旧 API 退役；提现与旧网关回执退役回归通过。

后台页面使用真实组件和隔离响应验证直接导入、权限隔离及桌面/窄屏布局。五网络解析仍由 RPC 模拟数据覆盖；没有连接正式节点进行真实转账，浏览器页面测试也不代表生产端到端验收。

本轮配置页与代理记录的命令、浏览器验证和门禁见[最新交付记录](../../.trellis/tasks/archive/2026-10/10-24-recharge-config-agent-records/research/delivery.md)。此前统一充值重构见[重构交付记录](../../.trellis/tasks/archive/2026-10/10-23-unified-recharge-flow/research/delivery.md)。上一轮限时引擎的完整记录见[初次交付](../../.trellis/tasks/archive/2026-10/10-23-timed-chain-deposits/research/delivery.md)，其中测试数量和包体数字仅代表那次交付。

上线前仍需应用新迁移、配置真实地址/节点，并逐网络验证实际转账、确认数、到账延迟、断线续扫和人工核对。本次没有生产部署或资金转账操作。
