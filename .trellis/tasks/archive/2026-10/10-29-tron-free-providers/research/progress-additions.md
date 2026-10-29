

## 2026-10-29 18:51（Europe/Istanbul） - Tron五家免费服务商核查与接入

- 完成内容：确认PublicNode、TronGrid、Ankr、Alchemy、Chainstack免费接入；PublicNode新增免Key公共备用身份，四个账号模板补免费额度/来源/日期，后台独立统计免费数量并排除QuickNode试用。保留主备、凭据、游标和完整到账证据规则，无新增迁移。
- 修改文件：wallet/domain/rpc_presets.rs、RPC预设回归、web充值预设/编辑器/测试与目录夹具、后台及后端规范、充值操作手册、本任务研究记录。
- 验证结果：PublicNode低频5次只读请求均200，主网与USDT精度通过，496笔交易/496回执完整匹配；Web充值101/101、typecheck/lint/fmt/diff通过。首轮Rust过滤器名称不匹配执行0项，已修正重新运行隔离MySQL预设回归；全量Web及浏览器验证进行中。账号模板仅官方文档确认，未使用真实Key、生产或实链付款。
- 后续事项：完成实库与静态检查、桌面窄屏保存验收和精确发布版本构建；无关代理管理WIP保留，未提交推送或部署。


## 2026-10-29 19:00（Europe/Istanbul） - Tron五家免费渠道最终验收

- 完成内容：PublicNode新增免Key备用，TronGrid/Ankr/Alchemy/Chainstack明确免费额度，后台显示6家可选其中5家免费；旧服务商/Key/优先级及主备冷却保持，QuickNode试用不计入免费。
- 修改文件：10-29-tron-free-providers/task.json.relatedFiles所列11个业务/测试/规范/进度文件及本任务研究记录。
- 验证结果：隔离MySQL预设2/2实际执行、Tron4/4、架构11/11、Clippy/fmt通过；Web全量1264/1264、充值101/101、类型/lint/policy15/coverage23通过。排除代理WIP的精确发布build通过，JS2805753/2600000仍超既有预算（本次增加1746字节）。Ego1728/1280目录、320保存、390冲突保留/放弃/只读通过，弹窗288/358px且无横溢，截图已查看。PublicNode样本496/496回执匹配；账号未使用真实Key验证。最后按官网将Alchemy免费速率更正为15次/秒，定向12/12、最终lint与精确build/typecheck复验通过。
- 后续事项：保存本地提交并归档，清理本任务浏览器/Vite/MySQL和临时导出；不推送、不部署。同版更新API/worker/后台后，管理员显式追加公共备用或填写免费账号凭据并保存，再观察实际持续监听。
