# 2026-10-22 排查记录

## 已核对
- `src/workers/seconds_contract_settlement.rs`：生产按数据库时间扫描 opened 到期订单，以 Redis 缓存价结算；失败日志包含 order_id/error，1 秒后重试。持久依赖故障可持续待结算，但尚未证明是本次原因。
- `src/modules/seconds_contract/infrastructure.rs`：Redis GET 有 2 秒超时；拒绝缺失、损坏、非正数或身份不符报价；结算及钱包/流水同事务写入。
- `src/main.rs`：需开关启用且 MySQL/Redis 初始化成功才启动结算协程。
- `src/config.rs`：结算开关默认 true；本地 .env 对应值也是 true，不代表线上配置。
- 移动端与 PC 存在到期状态刷新及权威状态对账，当前未复现前端卡单。

## 已执行验证
`node --experimental-strip-types --test mobile/tests/seconds-private-reconciliation.test.ts mobile/tests/seconds-live-multi-orders.test.ts pc/tests/seconds-reconciliation.test.ts`
14/14 通过，日志 `/tmp/seconds-pending-reconciliation.log`。本测试不验证线上 worker 健康。
本地 Docker daemon 无法连接，因此未获取运行容器日志；不据此判断生产服务状态。未运行需要真实 MySQL/Redis 的结算集成测试。

## 下一步
结合订单号查实际 status/expires_at/next_settlement_attempt_at/result，关联“秒合约结算失败，将按重试时间重新处理”或“秒合约结算批次失败”日志。先辨别未启动、批次失败、单单失败或仅显示未更新，再确定修复。

## 当前结论
根因未确认，无业务代码修改，无生产变更，无部署或推送。

## 2026-10-23 控制关联排查
用户确认线上环境，修改代理后台控制功能后出现。worker 在订单锁内读取订单级/用户级控制，并通过 resolve_settlement_outcome 计算结果；受控结果可改变 settlement_price，原 snapshot.price 另存 market_price。代理路由明确提供指定 win/loss 的控制。该分支确实参与自动结算，但没有线上错误，尚不能证明它是根因。未修复或增强指定输赢/改价能力；后续可排查真实行情结算、依赖和数据库一致性，需取得错误日志及明确具体改动。

## 2026-10-23 代理页面与部署核对
- SecondsOrderCountdown 仅在倒计时归零后显示待结算；两个代理订单页没有自动轮询，用户详情页缓存成功查询。现有测试明确要求倒计时不发请求。
- 用户确认整页刷新后仍待结算；不能据页面缺少轮询认定本次根因，未改动页面。需实际接口 status/expires_at 及服务端 error。
- src/main.rs 与 src/infra/mysql.rs 不执行数据库迁移；docker-compose.1panel.yml 定义独立 migrate 服务，API 依赖其成功完成。API 和迁移镜像应为同一版本，是否线上执行成功尚无证据。
- 只读日志入口（仅适用仓库 1Panel 容器命名）：docker logs --since 30m --tail 2000 hippo-exchange-api；docker logs --tail 100 hippo-exchange-migrate。没有执行生产命令，也不建议在未确认错误前修改资金/状态或盲目执行迁移。

## 已定位的失败点（用户提供线上日志）
2026-10-22T18:44:32.180Z，订单 51 报数据库列 0 解码类型不匹配：Option<i32> / INT 对 INT UNSIGNED；后续批次扫描 1、失败 1并反复重试。
迁移 0157 定义 seconds_contract_orders.price_precision 为 INT UNSIGNED NULL；infrastructure.rs 的 select_order_price_precision 按 Option<i32> 读取。worker 在结算结果和资金写入之前无条件调用该读取，故这处类型不匹配足以解释日志中的结算失败，并非单纯页面未刷新。尚未实连生产 DB 或执行修复回归，不能声称线上已恢复。
该字段用于指定输赢后生成受控判定价格；本轮只确认失败位置，不提供恢复操控能力的补丁。可继续处理按真实行情结算的路径。
