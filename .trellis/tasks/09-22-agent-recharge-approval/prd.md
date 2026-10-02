# 代理下级充值审核与免审授权

## Goal

代理可以对自己子树内用户提交后台人工充值，默认由管理员确认后入账；
管理员可以按代理授权免审，获授权代理的新申请直接入账。

## Requirements

- 沿用现有后台人工加币口径，入现货钱包可用余额，不扣代理钱包。
- 默认需审核；免审授权由独立管理员权限控制，仅当前代理有效，不继承。
- 授权变更有版本与原因，撤销即时影响新申请。待审核申请不会自动执行。
- 代理身份由令牌确定，范围与既有子树一致；提交、批准时事务内重新校验
  代理/账号/祖先状态和用户归属，禁止跨树及给代理本人加币。
- 充值申请金额/币种/用户/原因不可修改。状态 pending/credited/rejected；
  mode review/direct 表示提交时选择的审核路径。
- 提交幂等键绑定代理账号和完整请求；成功重放不重复入账。批准/拒绝是
  单次终态迁移，同审核人同动作同原因重放返回结果，其他终态操作冲突。
- 金额十进制字符串，校验资产精度和DECIMAL容量，溢出全事务回滚。
- 充值记录、钱包、流水、平台平衡分录、必需审计同事务；默认待审核不动钱。
- 代理有提交入口和自己的申请历史；管理员有审核队列及代理授权界面。
- 提交/审批未知结果重试保持同一请求身份；禁止双击/刷新重复加币。

## API Contract

- GET `/agent/api/v1/recharges/options` -> `{direct_recharge_enabled, assets:[{asset_id,symbol,precision_scale}]}`
- POST `/agent/api/v1/users/:user_id/recharge` -> RechargeRecord
  `{asset_id,amount:string,reason:string,idempotency_key:string}`
- GET `/agent/api/v1/recharges?status=&user_id=&limit=&offset=` -> `{recharges:RechargeRecord[],total}`
  仅当前代理账号发起的记录；当前已不在子树的用户记录不返回。
- GET `/admin/api/v1/agent-recharges?status=&agent_id=&user_id=&limit=&offset=` -> 同分页响应。
- GET `/admin/api/v1/agent-recharges/:id` -> 权威单条申请，仅查询终态恢复，不修改资金。
- POST `/admin/api/v1/agent-recharges/:id/review` `{decision:"approve"|"reject",reason:string}` -> RechargeRecord
- GET `/admin/api/v1/agents/:id/recharge-policy` -> `{agent_id,direct_recharge_enabled,version}`
- PATCH 同地址 `{direct_recharge_enabled:boolean,expected_version:number,reason:string}` -> 同授权响应。
- RechargeRecord: `id,agent_id,agent_admin_id,user_id,asset_id,asset_symbol,amount,
  reason,mode,status,reviewed_by,review_reason,created_at,reviewed_at,credited_at`。
  ID/版本为安全整数，金额string，时间毫秒，可空审核字段/time为null。
- 权限：后台队列 `agents.recharges.read`、审批 `agents.recharges.review`、
  授权写 `agents.recharges.authorize`；授权读沿用 `agents.read`。

## Acceptance

- [x] A01 后端迁移、权限、事务与精确幂等流程。
- [x] A02 代理申请/记录、管理员审批与授权界面。
- [x] A03 实库并发/越权/回滚及前端行为验证。
- [ ] A04 规范、OpenAPI、进度与最终门禁记录。

功能及行为测试已完成；A04最终发布验收仍受包体预算与截图验证限制。
规范/OpenAPI已更新。最终门禁及上线顺序见 `research/delivery.md`。

## Verification

覆盖默认申请不动钱、审核入账/拒绝、重复与并发批准、直接充值、撤销/
不继承授权、跨树/本人/改派、超精度/溢出、审计/分录失败回滚。
保留9月19日行情任务的现有改动；其包体预算已失败，必须单独记录增量，
不得提高预算、宣称全部发布门禁通过或顺便重做行情功能。

## Out Of Scope

代理钱包代付、自动赠送佣金、充值撤销/扣款、额度系统、多级审批、
线上充值/授权、提交推送部署及旧行情任务优化。
