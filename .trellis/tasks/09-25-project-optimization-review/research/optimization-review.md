# 项目优化复核：2026-09-26

## 范围与结论

基线为 `main@a2f1109` 加当前工作区既有改动；本轮开始时有 89 个未提交状态条目。继续已有 `09-25-project-optimization-review` 任务，只检查和记录，不实施产品改造、不操作实际资金、不迁移或部署。

保留六项可行动发现：三项业务正确性/并发风险、一项发布门禁缺口、两项性能优化。K 线聚合恢复和 CI 预算接线是本轮补出的具体缺口；PC 闪兑、代理交叉锁序边界、包体超限及行情性能问题属于重新核实仍开放的事项，不包装成全部新发现。

优先顺序：资金重试身份 → K 线恢复完整性与代理统一锁序 → 发布门禁 → 行情查询/运行开销。没有宣称发现线上损失或已经完成修复。

## R1 / P1：PC 闪兑没有确认所见报价，未知结果重试创建新交易身份

- 证据：`pc/src/views/Swap.vue:173-191` 维护展示报价，`pc/src/views/Swap.vue:195-221` 的按钮处理却重新请求报价并直接确认新 quote；失败只显示消息，不保存本次已提交的 quote。`pc/src/api/swap.ts:46-64` 请求金额由 number 转字符串，确认只有 quote_id。
- 金额另一路问题：`pc/src/views/Swap.vue:58` 先执行 Number，已经舍去的十进制尾数不会因 API 的 String(amount) 恢复。
- 服务端边界：`src/modules/convert/application.rs:109-113` 每次生成新 ID；`src/modules/convert/infrastructure.rs:437-446` 锁同一报价并阻止已消费报价重复确认。这个保护不合并两个不同报价；不是服务端同键重复扣账漏洞。
- 复现：运行本任务 `research/pc-swap-replay.mjs`，提取当前页面真实 handleSwap 函数，内存接口模拟第一次已经提交但响应丢失。两次点击提交 `new-quote-1`、`new-quote-2`，都不是展示中的 `displayed-quote`；输入 `1.000000000000000001` 到报价调用处成为 `1`。
- 影响：用户可能接受未展示的新报价；账户余额等条件仍满足时，未知结果重试可能再次兑换。尚未用真实资金或后端集成复现。
- 最小改进：金额端到端保留 DecimalText；确认用户看到的有效报价，过期或变化时重新展示确认；发送前持久化待确认报价身份，未知结果按原报价查询订单/恢复，不自动新建。后端对已消费报价返回冲突，因此恢复流程要核对既有订单，不把冲突直接当成成功。
- 验收：真实页面与适配器组合覆盖丢响应、刷新恢复、报价过期、报价变化、18 位小数和余额边界。现有 PC 闪兑相关 5 项测试本轮全通过，不能替代这类故障注入。

## R2 / P1：自动 K 线恢复会遗漏失败的高周期聚合，却推进到 live

- 证据：`src/workers/kline_recovery.rs:534-574` 只以兼容的 1m 缺根决定是否执行恢复；missing 为空直接跳过恢复，并推进检查点。`src/workers/kline_recovery.rs:685-729` 先写全部 1m，再逐个重建高周期，后阶段错误不撤销已写 1m。
- 终态证据：`src/workers/kline_recovery.rs:1035-1051` 把完整扫描置 live 并清错误；`src/workers/kline_recovery.rs:1002-1008` 不再选择已到终点且 recovery_status=live 的结束策略；失败标记只更新恢复状态，见 `src/workers/kline_recovery.rs:1068-1084`。
- 精确反例：策略仍为 active，区间 [00:00,00:05)，检查点 00:00，now=00:10。首轮五根 1m 成功而第一个 5m 聚合失败；次轮 1m 差集为空，检查点变为 00:04、状态 live，但 5m 仍缺失，后续不再选中。
- 本地证据：`research/kline-recovery-model.py` 两轮控制流模型输出 roots=5、aggregates=0、checkpoint=4、status=live、selected_next_round=false；无故障控制组有一根 5m。模型复刻分支，不是原生 Rust 或真实 Mongo/MySQL 故障注入。
- 最小改进：把待聚合窗口作为独立恢复工作，按扫描范围或持久化待完成窗口驱动，而非只依赖本轮新补 1m；聚合确认完成后再推进检查点，保留版本、来源和普通/人工历史优先规则。
- 验收：实库注入“全部 1m 成功、首个聚合读/写失败”，重启后补齐聚合才变 live；补正常、部分 1m 失败、版本变化及普通历史保护控制组。

## R3 / P1：代理充值与旧创建/改派入口存在相反祖先锁序

- 证据：`src/modules/agent/infrastructure/recharges.rs:63-67` 先锁用户；`src/modules/agent/infrastructure/recharges.rs:83-123` 用 BTreeMap 按代理 ID 升序逐行 FOR SHARE。旧入口 `src/modules/admin/infrastructure/agents.rs:202-229` 先锁目标子代理 FOR UPDATE，再锁祖先。调用点见 `src/modules/admin/application/agents.rs:80-84` 和 `src/modules/admin/application/agents.rs:264-266`。
- 触发条件：两笔操作针对不同用户，但共享 root→child 代理链。充值已持 root(S) 后等待 child；旧创建/改派已持 child(X) 后等待 root，形成等待环。同一用户会先被共同用户锁串行化，所以不能用同用户测试否定此问题。
- 本地证据：已有 `research/lock-order-model.py` 的不同用户控制流有一个死锁状态；同用户、统一顺序两个控制组均无死锁。主会话读取并核对了双方源码，独立检查者实际执行三项模型断言。不是本轮 MySQL 压测结果，不宣称账务部分提交或线上死锁频率。
- 最小改进：统一所有代理写入口的锁集合与顺序，预读只用于确定有界 ID 集合，加锁后重新核对层级；旧入口不再先锁叶节点再回锁祖先。重试可作为事务级兜底，不能替代消除反序。
- 验收：隔离 MySQL 8.4 上用屏障稳定重现不同用户充值×创建/改派交错，统一顺序后重复压测，同时保留权限、层级变化与资金原子性断言。

## R4 / P1：后台包体门禁当前失败，主 CI 又没有执行它

- 新构建结果：`npm --prefix web run build` 退出 0。随后 `npm --prefix web run budget` 退出 1，全部 JS raw=2,608,723 > 2,600,000，超 8,723 字节。按同样 gzipSync 口径统计 gzip=769,563 > 750,000，超 19,563 字节。预算脚本遇到 raw 超限先抛错，因此后一个数来自独立产物统计。
- 证据：`web/bundle-budget.json:10-12`、`web/scripts/check-bundle-budget.mjs:40-57`。该限制统计所有脚本，不等于用户首屏实际下载量或页面性能实测。
- CI 证据：`.github/workflows/docker-image.yml:60-67` 委托 P0 脚本并设置依赖；`scripts/p0-release-gate.sh:23-31` 中后台仅跑 lint、typecheck、test，没有 build/budget。PC 部分也只调用 test:margin，不能把这个入口的绿灯解释成所有 PC 场景通过。
- 影响：本地交付条件明确未达到，但当前主质量流水线没有对应自动检查，容易出现“CI 绿而包体验收失败”。并未查询本次远端运行状态。
- 最小改进：先查清依赖与打包重复、收敛实际字节，再将同配置 build→budget 纳入后台 CI。仅拆 chunk 不保证降低全部 JS 总量；不要直接提高阈值绕过失败。
- 验收：同一构建配置下预算通过，CI 合同测试证明 build/budget 是发布前依赖；分别跟踪初始、异步和总包体，不混用口径。

## R5 / P2：行情健康查询仍对无时间窗历史做双 MAX 聚合

- 证据：`src/modules/admin/infrastructure/market_feed.rs:102-115` 按 symbol/source 筛选，对全部保留历史分别 MAX(observed_at)、MAX(ingested_at)；`src/modules/admin/infrastructure/market_feed.rs:131-168` 随后串行读取两个恢复统计。
- 现有 tick 索引见 `migrations/0114_event_time_price_snapshots.sql:10-12`。本轮未执行 EXPLAIN，不将查询文本直接等同于已证实的全表扫描；过去慢日志也不作为当前耗时。
- 优化：考虑维护按 symbol/source 的最新状态读模型，或分别以可用索引获得两个最大值，再用生产同版本的 MySQL 8.4 执行计划和代表性数据验证。最后事件时间与最后摄取时间是两个不同语义，不能简单改取最新事件所在行的摄取时间。
- 验收：有界查询成本、乱序/补录数据正确性、断流诊断和多来源隔离。不要为缩短查询直接删除仍被结算引用的 ticks。

## R6 / P2：行情循环重复读取历史，并把交易触发串在广播前

- 证据：`src/workers/synthetic_market.rs:966-1007` 每次最多读取 24 小时的 1,440 根 1m，并解析五个 Decimal 字段；调用在 `src/workers/synthetic_market.rs:373-378` 和 `src/workers/synthetic_market/default_market.rs:272`。
- 广播依赖：`src/modules/market/infrastructure/adapters/ingestion.rs:268-273` 串行 await 现货、杠杆触发后才发布。公平候选轮转已修，不再重复报告旧 ID 饥饿；这里关注单项与整轮执行时间。
- 优化：先增加 round/pair/history/trigger 分段时延及 observation_age 指标；按分钟缓存并增量更新历史窗口，明确恢复补写/版本变化的缓存失效；据实测决定有界并发或持久化触发消费者，保留每交易对顺序、租约 fencing 与资金幂等。
- 验收：代表性历史规模、热交易对与多交易对压测，监控尾延迟、租约年龄和 Mongo 读取量。不要用无追踪后台任务直接绕过归档/触发顺序。当前没有 QPS 或时延实测，不承诺具体提速百分比。

## 本轮验证与限制

| 验证 | 结果 | 证明范围 |
| --- | --- | --- |
| PC handleSwap AST 提取 + 内存响应丢失 | 脚本通过，观察到两个新报价和金额尾数丢失 | 当前函数行为，非后端资金集成 |
| PC 既有 convert/swap 定向测试 | 5/5 通过 | 现有映射/接线合同 |
| Cargo lib: kline_recovery / financial_retry / synthetic_market_scheduling 过滤 | 实际匹配 17 项，17/17 通过 | 恢复与退避既有单元测试，不代表并发或DB故障注入 |
| 代理锁序状态枚举 | 3 组断言通过 | 简化锁模型，无真实数据库 |
| 恢复部分写入模型 | 故障/正常 2 场景通过 | 按源码分支构造的模型，不执行 Rust |
| 后台当前源码 build | 退出 0 | 类型和生产构建；有既有大块/第三方 eval 提示 |
| 后台新产物 budget | 退出 1 | 当前总 JS raw 超限，独立统计同时确认 gzip 超限 |

Cargo 首次因 PATH 缺失退出 127；确认本机 Cargo 链接后加入 `.cargo/bin` 重跑成功。Rust 测试有一条清理旧增量对象的文件不存在警告，测试仍退出 0。模型首次尝试的 apply_patch 命令在当前环境缺失，之后用文件工具创建；未因此修改任何生产文件。

本轮不运行全部 Rust/三端门禁、不启动 MySQL/Mongo/Redis、不重跑浏览器，不操作远端或生产。MySQL 8.4 迁移演练、真实并发和外部资金链路仍为后续验证，不从历史测试推导当前通过。

本轮明确不重新列为缺失：已交付的代理分币种精确汇总、移动端 DecimalText、后端金额容量守卫、平台分录、贷款/理财额度、公平候选调度、人工成交审计。
