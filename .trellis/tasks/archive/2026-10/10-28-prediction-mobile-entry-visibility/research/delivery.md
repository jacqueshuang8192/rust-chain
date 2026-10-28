# 竞猜手机端入口开关交付记录

- 后台路径：竞猜管理 → 竞猜配置 → 全局策略 → 手机端显示竞猜入口。
- 默认开启，关闭后保存全局策略。手机端进入产品中心或回到前台读取最新配置，入口及占位一起收起；新闻/帮助仍可用。
- 仅控制发现入口；PC、直接路由、同步、下注、订单和结算未更改。旧后台省略新字段保留当前值。
- 发布顺序：执行 `0164_prediction_mobile_entry.sql` 后更新后端、后台和Mobile。未部署或推送。

## 验证

- 隔离MySQL 9.3，`DATABASE_URL=<isolated> cargo test --test prediction_commission_routes -- --test-threads=1`：7/7，覆盖本开关HTTP读写、空/非空资产公共配置、旧请求、409、审计与原有资金回归。SQLx迁移重入通过；列默认true且非空。最终移除临时浏览器服务后新增测试再次1/1通过。
- `cargo test --lib prediction`：11/11；`cargo test --test backend_architecture`：11/11；最终 `cargo check --all-targets`、生产 `cargo clippy --lib --bins -- -D warnings`、`cargo fmt --all -- --check` 通过。
- Web typecheck、lint、全量126文件1212项、production-policy15项、coverage23项、build通过。最终UI微调/临时夹具清理后typecheck、lint及竞猜/审计/中文展示3文件33项通过。
- Mobile `npm run release:gate` 全通过：923/923、两种网页构建、产物检查、包体预算、源码和测试质量检查；初轮定向21/21。
- Ego浏览器使用生产组件及真实本地API/隔离数据库，验证后台关闭→保存→重载持久化、Mobile隐藏→后台恢复→Mobile重开显示、再次关闭后Mobile返回前台无重载隐藏。后台1728/390/320、手机390及320/448深色英文无水平溢出；隐藏列表64px、显示列表128px，新闻与帮助保留。截图均已查看。
- `git diff --check`、Trellis任务校验通过。

## 既有门禁限制

- Web全部JS 2,785,097 bytes，预算2,600,000；前一任务记录基线2,784,310，本轮+787。未调高预算。
- 全量backend_documentation失败：未修改且与HEAD一致的钱包 `scanners/provider_runtime.rs` 的 `interval`、`retry_after` 缺中文doc。未修改无关模块或屏蔽检查。
- 本轮未进行生产部署、真实资金操作或原生设备构建。

## 清理与范围

- 临时Rust HTTP服务、Web/Mobile Vite、浏览器任务空间、测试令牌及前端/Rust临时夹具已清理；隔离数据库已关闭。
- 本任务文件范围见task.json.relatedFiles。已有代理管理界面、其规范和进度记录、两个诊断任务与.vite保持原样，不纳入本功能提交。

本地功能提交：`8c493a7`。
