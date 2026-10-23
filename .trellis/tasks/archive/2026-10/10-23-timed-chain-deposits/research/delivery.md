# 限时链上充值交付记录

日期：2026-10-23（Asia/Hong_Kong）。实现和本地业务验收已完成；后续按用户授权保存功能提交 `c453041` 并归档实现任务。推送核验另记于 `docs/superpowers/PROGRESS.md`。未部署或进行真实链上转账，发布前仍须解决下述既有门禁并完成实链验收。

## 已交付行为

- 用户先输入币种数量，再创建订单。全链路使用精确十进制文本及最小币单位；不进行法币换算，不添加尾数。
- 按网络、链上资产身份、地址和精确金额占用 30 分钟。不同金额共用地址，同金额换地址；满池提示重试，地址数量可扩容。已付款组合也保持至原截止时间。
- 数据库网络行锁保证并发分配，在锁内读取时间；用户锁保护跨网络幂等。重试重放原订单，不能重置到期时间或改变金额、币种和网络。
- 到期释放组合；迟扫按历史链上付款时间匹配。有未解决过期未付历史的复用组合、错额、分笔、迟付及同秒先后不明的款项进入人工核对。
- 链证据去重且不可变。自动与人工入账均复用钱包余额、充值流水、账本和平台分录事务；人工入账按实际到账量扣原手续费，保留原应付数量，并要求明确目标订单和审核依据。
- Ethereum、Base、Tron、Bitcoin、Solana 均有只读监听适配器、持久游标和恢复保护。配置版本在入账事务持有网络锁后再次核验，过期的在途扫描不能继续写入资金。
- 旧永久绑定地址与新共享池隔离。停用资产或地址只关闭新订单；历史证据和监听身份保留。深度重组或有历史订单但游标丢失时暂停，禁止静默跳到最新高度。
- Mobile/PC 已接入金额输入、订单恢复、服务端倒计时、轮询、历史分页及状态展示；过期或待核对状态隐藏支付二维码和复制入口。后台支持节点、链上资产、共享库存、订单筛选和人工核对，沿用权限与审计。
- PC 使用现有登录会话标识，兼容非 JWT 令牌；旧会话的迟到响应不能污染当前充值状态。

## 验证记录

以下为本任务实际执行结果。RPC 与浏览器使用受控模拟响应，不能视为实链或生产验证。

### 后端

隔离实例为 `127.0.0.1:13367/timed_deposits_test`。资金测试显式设置测试数据库环境变量，未使用生产数据库。`0158`、`0159` 两份新迁移已实际应用，重复运行迁移通过；后续不得修改已应用迁移。

| 命令/范围 | 结果与证据 |
|---|---|
| `TIMED_DEPOSIT_TEST_DATABASE_URL=mysql://root@127.0.0.1:13367/timed_deposits_test cargo test --lib timed_deposit -- --nocapture` | 13/13，通过金额/身份单元测试、RPC fixture 及一项覆盖完整场景的真实 MySQL 集成测试；`/tmp/timed-deposit-rust-test.log` |
| `DATABASE_URL=mysql://root@127.0.0.1:13367/timed_deposits_test cargo test --test wallet_routes timed_deposit -- --nocapture` | 1/1，实际 HTTP 路由的金额类型、用户隔离、订单筛选与后台权限；`/tmp/timed-deposit-route-test.log` |
| 同一隔离库运行 `wallet_routes` 的 `wallet_deposit_` 与 `wallet_base_deposit_` 过滤项 | 3+1 全部通过，覆盖旧永久分配、充值开关、事件幂等/冲正、Base 复用 ETH 地址池；`/tmp/timed-deposit-legacy-routes.log` |
| `cargo test --test openapi_routes timed_deposit` | 1/1，金额字符串、UTC 毫秒、鉴权、订单号参数及敏感字段不暴露 |
| `cargo check --all-targets --all-features` | 通过；`/tmp/timed-deposit-check-final.log` |
| `cargo clippy --lib --bins --all-features -- -D warnings` | 通过；`/tmp/timed-deposit-clippy-final.log` |
| `cargo test --test backend_architecture --test backend_documentation` | 11/11 与 1/1；`/tmp/timed-deposit-architecture-final.log` |
| `cargo fmt --all -- --check` | 通过 |

实库集成覆盖 12 个并发请求争用 10 个同金额名额、异金额共址、到期/幂等、历史付款窗口、歧义转人工、重复事件、手续费和账务一致性、审计失败回滚、同秒人工核对、配置变更拒绝旧扫描、游标续扫和异常暂停。13 项不是 13 个独立数据库测试；其中真实 MySQL 用例包含上述多组断言。早先钱包单元测试 83/83 未设置数据库环境，其静默跳过项不作为实库证据。

RPC fixture 覆盖 EVM 完整成功调用树与失败祖先、ERC20 日志身份、Tron 成功且完整的 TRC20 回执、BTC 各 vout 独立匹配及 coinbase 排除、Bitcoin Core Basic Auth 请求头、Solana 原生/经典 SPL 与 inner instructions、临时 token 账户和严格 slot 范围。

### 前端与后台

| 命令/范围 | 结果与证据 |
|---|---|
| `npm --prefix mobile run release:gate` | 全门禁通过，901/901，类型、PWA/Tauri Web 构建、制品、包体与治理通过；`/tmp/timed-deposit-mobile-gate.log` |
| `node --test --experimental-strip-types mobile/tests/timed-deposits.test.ts` | 最后新增 PC 非 JWT 令牌回归后 6/6，通过真实两端面板 SFC 及真实 PC Recharge setup；`/tmp/timed-deposit-ui-final.log` |
| `npm --prefix mobile run type-check:tests` | 新增回归后再次通过；未把该定向结果冒充重新执行的完整 902 项门禁 |
| `npm --prefix pc run type-check` 与 `npm --prefix pc run build` | 最终均通过；构建见 `/tmp/timed-deposit-pc-build.log` |
| `npm --prefix web test` | 119 文件、1137/1137；`/tmp/timed-deposit-web-test.log` |
| `npm --prefix web run typecheck` 与 `npm --prefix web run lint` | 最后生产代码修改后均通过 |
| `npm --prefix web test -- src/admin/deposits --maxWorkers=2` | 最后生产代码修改后定向 5/5；`/tmp/timed-deposit-web-focused-final.log` |
| `npm --prefix web run test:production-policy` | 15/15 |
| `npm --prefix web run test:coverage` | 23/23，通过既有指定覆盖率范围 |
| `npm --prefix web run build` | 最终构建通过；`/tmp/timed-deposit-web-build-final.log` |
| `npm --prefix web run budget` | 未通过，具体基线与增量见下文；`/tmp/timed-deposit-web-budget-final.log` |

Mobile 包体：PWA JS 1506.1 KiB raw / 498.8 KiB gzip；Tauri Web JS 1502.5 / 497.7；CSS 654.9 / 124.6。清理旧页面约 2.1 KiB 无效 CSS；本特性明确将全路由 raw JS 总预算由 1500 调至 1510 KiB，入口、gzip、CSS 预算不变，已同步规范。

### 实际浏览器交互

- 使用任务独占 Ego TaskSpace 8 和仓库内 fixture，响应完全隔离。Mobile 测试实际组件，PC 测试真实 Recharge 页面及 Pinia 登录，后台测试真实设置和核对页。
- 输入 `9007199254740993.000000000000000001` 后，Mobile/PC 创建请求保持原精确字符串；PC 使用非 JWT 测试令牌成功创建订单。
- Mobile 已生成订单/二维码状态在 320、390、448 px 的明暗六组均无横向溢出；到期和待核对时隐藏二维码与复制。
- PC 1280 px 页面无横向溢出。后台确认数从 12 保存为 15，敏感字段仍为空；明确读取目标订单并填写依据后完成模拟人工入账，390 px 核对弹窗无横溢。
- 代表截图：`/tmp/timed-deposit-mobile-390-light.png`、`/tmp/timed-deposit-mobile-390-dark.png`、`/tmp/timed-deposit-pc-1280.png`、`/tmp/timed-deposit-admin-review-390.png`。
- 未验证真实节点、真实转账、生产鉴权会话或原生设备安装包。

## 已确认的仓库既有门禁问题

1. **后台总 JS 预算**：最终 2,743,238 bytes，限制 2,600,000。通过独立临时目录 `git archive HEAD web` 构建的修改前基线为 2,721,886 bytes，本次增加 21,352 bytes。基线已超限，本次未提高后台预算。基线日志 `/tmp/timed-deposit-web-baseline.log`，目录记录 `/tmp/timed-deposit-web-baseline-path`。
2. **全目标严格 Clippy**：`cargo clippy --all-targets --all-features -- -D warnings` 在既有 `tests/unit_src/src_modules_seconds_contract_tests.rs:316` 的 `settlement_price > BigDecimal::from(0)` 触发 `cmp_owned`。该文件相对 HEAD 无修改；生产 lib/bins 严格检查通过。
3. **金融 DTO guard**：`cargo test --test numeric_contract` 因既有 `src/modules/new_coin/presentation/issuance.rs` 的 `TradeQuoteRequest.quantity` 及 `TradeQuote` 数量/价格/金额字段未使用边界反序列化而失败。该文件相对 HEAD 无修改；日志 `/tmp/timed-deposit-contract-tests.log`。未扩大本任务修改范围。

## 配置与上线边界

具体步骤见 `docs/superpowers/timed-chain-deposits.md`。上线需要执行迁移，配置 `CREDENTIAL_ENCRYPTION_KEY`、真实收款地址、可信节点、明确的链上合约/mint 和精度，等待监听追上区块。程序不会生成虚构收款地址，也不接收私钥。

- Ethereum/Base：原生 ETH 要求支持 `debug_traceBlockByHash` 的 callTracer；代币需明确 ERC20 配置。
- Tron：本批仅 TRC20，不开放原生 TRX。
- Bitcoin：主网 Bitcoin Core 完整历史区块读取和只读 RPC。
- Solana：SOL 与经典 SPL Token，暂不开放 Token2022。
- 发布前需逐网络受控转账，核验确认数、到账延迟、断线续扫和人工归属流程；本地模拟通过不能替代这些验收。

## 收尾状态

Ego TaskSpace 8 已关闭。仅停止本任务的 Vite 15173/15174/15175 和隔离 MySQL 13367，`lsof` 确认均不再监听。隔离数据库目录和 `/tmp` 日志保留诊断；未清理其他项目服务、既有 `.vite/` 或两项独立诊断任务。

最终执行 `git diff --check`、任务 `task.py validate`，并额外检查新未跟踪文件的冲突标记及尾空白。功能已保存为提交 `c453041`，实现任务已按用户授权归档；提交后的推送核验见进度记录。未进行生产部署。
