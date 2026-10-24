# 链上充值扫描配置交付

## 实现

参考 GMPay Edge 固定提交 `5c32e35f8b92625e7f03040dd5b5c2c154ef9af7` 的分层配置与就绪原因设计。管理页增加节点、监听币种、收款地址、扫描与接单四步指引，每个网络/币种支付方式显示完整阻塞原因。

配置读取增加 `scan_fresh`、`cursor_height`、`checked_at`。MySQL 按现有准入的 120 秒窗口判断新鲜度，读取持久游标；不额外探测节点、不更改游标、不回传凭据。Alchemy/Helius 增加服务商密钥输入并 URL 编码生成原节点地址，沿用加密、revision 和显式保存。没有新增迁移。

页面可见、无编辑/确认/读取/写入时每 15 秒刷新；地址确认通过可选 `ConfirmAction.onOpenChange` 暂停刷新，失败保留确认框。地址专用角色保持原权限隔离。保留快速/链上两方式、五网现有扫描能力、精确币种数量、30 分钟组合和歧义人工核对。

## 验证

- 前端最终 typecheck、lint、build 通过；全量 126 文件 / 1190 项通过（375.74 秒）。
- 先前全量 126 文件 / 1189 项通过；补充轮询测试后的定向 3 文件 / 34 项通过。最终类型检查发现测试使用超出当前 TS lib 的 `findLastIndex`，改为现有支持的 map/lastIndexOf，未改编译目标。
- production-policy 15/15；coverage statements 85.61%、branches 81.78%、functions 85.33%、lines 92.87%。
- 隔离 MySQL 实际运行 `timed_deposit_settings_expose_real_scan_snapshot_without_credentials`，1/1 通过（非跳过）：null/fresh/stale、持久高度、DB检查时间、凭据不回传、读取不改游标。
- `cargo test --test backend_architecture --test openapi_routes` 11+20 通过；`cargo clippy --lib --bins -- -D warnings`、`cargo fmt --all -- --check` 通过。最终格式后再次执行契约检查 11+20 通过。
- `git diff --check`、Trellis task validate 通过。
- 包体预算仍失败：JS 2,760,903 bytes，超过既有 2,600,000 门槛；上轮 2,754,677 bytes，本轮增加 6,226 bytes。未放宽门槛，未扩大范围改造全站拆包。

## 浏览器

Ego Browser 使用真实本地 API、隔离 MySQL 和 Vite；未运行链扫描 worker，也没有连接外部节点或真实转账。

- 五个网络切换均显示对应节点能力、币种边界和进度单位。
- Alchemy 预设填写测试密钥可生成 URL，Bearer 为空；编辑期间禁用刷新，取消放弃无保存。
- 通过隔离数据库夹具验证 5/5 就绪、过期后 0/5，以及扫描新鲜但暂停接单时 0/5 和每个币种明确的暂停原因。以上只验证状态呈现，不代表实际扫描主网。
- 1728、390、320px 均无页面横溢；桌面四列，窄屏一列。截图已实际查看。修正 Semi Tag 为 div 导致摘要 flex 规则误拉伸的问题。
- 320px 可打开地址导入、输入、取消及放弃草稿；刷新在编辑期间禁用，关闭后恢复。
- 最后一次暂停状态的精确文本等待因 DOM 将文本拆为多个节点而超时；随后通过实际 DOM 和快照确认显示及全部阻塞原因正确，无业务故障。
- 浏览器空间已 finish；数据库夹具 eth 开关/时间/错误/游标已恢复；临时 Rust 启动夹具已删除且该文件对 HEAD 无差异；会话 token 文件已删除，API/Vite/MySQL 已关闭。

## 发布边界

后端和后台前端需同版发布；旧响应缺少扫描快照字段会被严格校验拒绝，不能假装正常。现有数据库无需本轮新迁移。

没有生产部署、真实节点鉴权/额度验证或实链到账验证；保存节点后仍需等待真实 worker 追上。没有新 WSS/Webhook、交易所接入、法币报价、自动跳过历史缺口、二维码改造或私钥处理。本轮仅本地提交，不推送。
