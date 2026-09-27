# 多国家配置第一批交付记录

记录时间：2026-09-27 07:50 HKT（本机时钟）。

## 实施范围

仅实现上一轮 F1–F3；F4–F6、国家标识迁移及地区支付不在本批范围。

1. 贷款名称、理财介绍及理财分类保留已有 locale；国家默认语言变化或切换国家不再重写原语言。只有空语言才按国家默认初始化，支持显式选择内容语言，选择器保留旧值。未实现自动翻译，也未改首条决定 `default_locale` 的既有规则。
2. 启用 KYC 时，非空证件规则必须覆盖所有允许国家；额外规则允许保留。后台提示缺失国家并阻止保存，后端领域校验拒绝绕过；停用可保留旧跨清单冲突，单条规则的原有校验不放宽。手机与 PC 使用有效交集，空交集不回退到全部国家，并阻止无效国家提交。保留空白名单/空规则语义和原始国家身份，不把 CN 自动等价为 China。
3. 国家目录每页 100，按实际已读取条数续页，最多 100 页；失败、重复身份、total 变化、缺页或取消均不返回部分结果。新闻/理财/贷款共享完整目录，KYC 复用同一查询函数。无 total 的旧响应兼容短页结束；该模式不能额外证明服务端未隐式截断。

## 修改文件

- `web/src/api/adminCountries.ts`、`adminCountries.test.ts`
- `web/src/admin/resources/actions/{shared,loan,earn}.tsx`、`loan.test.tsx`
- `web/src/admin/resources/resourceConfigs.test.tsx`
- `web/src/admin/actions/{KycManagementPage.tsx,KycManagementPage.test.tsx,kycCountryPolicy.ts,kycCountryPolicy.test.ts}`
- `web/src/shared/adminErrorMessage.ts`
- `src/modules/kyc/domain.rs`、`tests/unit_src/src_modules_kyc_domain_tests.rs`
- `mobile/src/views/KycView.vue`、`mobile/src/i18n/messages/{en,zh-CN}.ts`、`mobile/tests/kyc-country-policy.test.ts`
- `pc/src/views/User/KYC.vue`、`pc/src/i18n/index.ts`
- `.trellis/spec/admin/resource-response-contract.md`、本任务目录、`docs/superpowers/PROGRESS.md`

以上为本批范围，不是工作区全部未提交文件；原有其他功能改动保留。

## 最终自动验证

| 命令 | 结果 |
|---|---|
| `npm --prefix web run test` | 103 文件、984/984 通过 |
| `npm --prefix web run lint` | 通过 |
| `npm --prefix web run build` | 类型与生产构建通过；依赖 eval/chunk 警告保留 |
| `npm --prefix web run test:production-policy` | 15/15 通过 |
| `npm --prefix web run test:coverage` | 23/23 通过，既有门禁通过；不是本批全部代码覆盖率 |
| Mobile `type-check`、`type-check:tests`；PC `type-check` | 全部通过 |
| Mobile KYC policy/country-search/document-type-search/i18n 定向 `node --test --experimental-strip-types` | 18/18 通过；两端交集用真实 computed AST 抽取隔离执行，非实机认证 |
| `cargo test --lib --quiet` | 507/507 通过，含新增 KYC 4/4 |
| `cargo test --test backend_architecture --test backend_documentation --quiet` | 架构 11/11；文档门禁失败：既有 `src/modules/convert/service.rs:92 route_offset` 缺少中文 doc，本批未改该函数 |
| `cargo fmt --all -- --check` | 独立执行通过 |
| `git diff --check`、`task.py validate` | 通过 |
| `npm --prefix web run budget` | 失败：JS raw 2,611,227 > 2,600,000；另只读统计 gzip 770,472 > 750,000 |

包体历史基线为 raw 2,608,723 / gzip 769,563，本批增加 raw 2,504 / gzip 909。没有提高预算或改写既有失败。最初新贷款测试使用不匹配 Semi DOM 的 combobox/name 选择器而失败，改为现有 label/.semi-select 定位后，全量重跑通过。首次架构测试目标名称不正确，改用实际 `backend_architecture` 后通过。

## 浏览器证据与边界

- Ego TaskSpace 4、Vite 5198，本地页面内 mock 拦截，不连接真实资金写入。验证贷款国家默认 en 时原语言仍为 zh-CN；键盘显式改为 en 后原中文标题不变。观察国家请求 offset=0、100。
- KYC 允许 China/Japan 但仅 China 规则时，显示缺失 Japan 且保存禁用；将允许国家改为 China 后提示消失、保存启用，mock writes=0。
- KYC 在 1728/390/320px 页面横向溢出为 0；`kyc-desktop.png`、`kyc-390.png` 已截图并查看。贷款抽屉三宽度自身横向溢出为 0；已展开下拉在直接切换视口时出现旧位置溢出，关闭后 320px 页面溢出归零，未宣称所有下拉响应式情况通过。
- 两次下拉指针定位/可见性失败，键盘操作成功；不能据此声称指针验收完整。mock 的 CDP 新文档注册需在导航调用中重新安装，一次遗漏后权限加载失败，修正 fixture 后成功；失败轮次未提交配置。
- Vite 记录一次无业务栈 `Script error` 及 hydration fallback 警告；未完成无控制台错误验收。不是实时后端集成、数据库回归或原生设备验收。
- TaskSpace 4 已关闭，本任务 5198 进程已停止；未操作其他服务或用户浏览器空间。

## 后续

1. 发布前解决已有包体/文档门禁，补真实 API、指针操作与控制台验证。未执行全端完整发布门禁或数据库集成，不等同可上线。
2. 下一批仍为手机国家语言偏好、产品多语言内容选择、公告地区筛选（F4–F6）。
3. 本批无迁移、生产配置、真实资金操作、提交、推送、部署或自动归档钩子。

## 用户授权提交后的独立快照验证

2026-09-27 用户明确要求提交推送。仅暂存本批及前置审查记录，共36个文件；代理充值、行情、历史分页与临时缓存保持未提交。共用语言文件只暂存 KYC 提示，PROGRESS 只暂存本次审查、实施与提交记录。

从暂存树 `974997018f585c1852d8010a937ff4b5d57f31dd` 导出隔离目录，复用本机依赖但不读取未提交源文件；后续仅补本段等文档记录，业务源码不变。Web类型构建通过、受影响5文件103/103测试通过；Mobile/PC类型与测试类型通过、KYC定向18/18通过；Rust KYC领域4/4及全局fmt通过。

独立快照包体仍未通过：JS raw2,562,479，gzip750,656，比gzip750,000预算超656字节。此数值与前述含其他任务的完整工作区不同，均为真实检查结果；未抬阈值或绕过hook。前述Convert文档失败来自未纳入提交的分页改动，本轮不宣称独立快照全局文档门禁已重跑。普通推送会触发既有镜像CI，提交不表示发布验收完成。
