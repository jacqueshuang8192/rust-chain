# 多国家后台配置复核

日期：2026-09-26。范围为当前工作区的 Admin、Rust 后端、Mobile 和代表性 PC 消费链路。仅审查，未修改业务代码、数据或线上配置。

## 已确认问题

### F1 / P1：编辑产品会改写已有内容的语言身份

- `web/src/admin/resources/actions/loan.tsx:124` 的 `syncLoanProductCountryLocales` 将每条内容的 locale 强制替换成国家当前默认语言，保留原 title。
- `web/src/admin/resources/actions/loan.tsx:446` 在打开编辑页且国家目录加载后调用它。改过国家默认语言后，仅修改利率再保存，也可能把旧中文标题标成英文；同一国家多语言条目也被压成同一种语言。
- 理财介绍和分类有相同模式：`web/src/admin/resources/actions/earn.tsx:234`、`:260`，编辑入口在 `:535`、`:817`。
- 修复方向：国家默认语言只初始化新条目；编辑保留每条原 locale，提供国家支持语言选择，明确选择默认内容；切换国家/语言必须有内容影响提示，不能自动当作翻译完成。
- 证据：抽取当前贷款同步函数执行，输入中文身份与英文国家默认值，输出 locale=en 而原文不变。未执行数据库写入。

### F2 / P1：KYC 允许国家与证件规则可以互相矛盾

- `src/modules/kyc/domain.rs:139`、`:144` 分别校验允许国家和证件规则，没有校验两者交集是否可用。
- `mobile/src/views/KycView.vue:74` 优先使用证件规则国家，有规则就忽略 allowedCountries；后端 `src/modules/kyc/domain.rs:182`、`:190` 要求同时通过允许国家与证件规则。
- 示例：允许 CN，但只配置 US 证件规则；手机显示 US，提交必被国家白名单拒绝，CN 也无法在该配置下完成认证。只在这种冲突配置下发生，不是全部 KYC 不可用。
- 修复方向：保存时校验可用交集与规则完整性，提供冲突说明；手机端按同一有效集合展示。空白名单与空规则的既有语义必须明确保留。
- 证据：执行当前 computed 源码，得到 US；拒绝路径通过 Rust 源码核对，未运行实库提交。

### F3 / P2：后台国家下拉超过 100 个会截断

- `web/src/admin/resources/actions/shared.tsx:242` 请求 limit=500，`web/src/admin/actions/KycManagementPage.tsx:299` 请求 limit=200。
- `src/modules/admin/application/system_config.rs:545` 使用统一上限，`src/modules/admin/application.rs:388` 把 limit 限定到 100。
- `web/src/api/adminResources.ts:68` 只发一次请求，不自动续页；两个调用方也不根据 total 续取。超过 100 个活动国家时，新闻、理财、贷款和 KYC 配置选项会缺尾部国家。
- 修复方向：统一国家目录查询器，按真实响应和 total 有界续页，或者服务端远程搜索；不能仅把前端 limit 再调大。需加入超过 100 个国家的回归。
- 证据：跨层静态调用链；未创建 101 条数据库记录。

### F4 / P2：手机端丢弃后台国家语言配置

- 后端 `src/modules/user/presentation.rs:29` 返回 preferred_locale、default_locale、supported_locales；`mobile/src/api/user.ts:101` 未映射这些字段。
- `mobile/src/api/auth.ts:161` 的国家适配器仅保留 code/name，丢掉默认语言和支持语言。
- `mobile/src/i18n/index.ts:22` 只使用本地存储、系统语言和中文兜底；`mobile/src/views/LanguageView.vue:38` 显示固定全局语言列表。
- 影响：后台配置某国家仅支持英文或默认英文，不会按该策略初始化手机端；PC 已有 `pc/src/stores/setting.ts:33` 与 `pc/src/api/backendAdapters.ts:989` 的账户语言解析，两个端语义不一致。
- 修复方向：定义用户手动选择、账户偏好、国家默认、系统语言的优先级并接入手机端；明确 supported_locales 是限制还是建议，避免每次登录强制覆盖用户选择。
- 证据：当前国家适配器执行后仅剩 code/name；其余为源码核对。

### F5 / P2：手机端没有消费产品多语言内容

- `src/modules/loan/presentation.rs:317` 返回 name_json；`mobile/src/api/loan.ts:62` 只用 product.name。
- `src/modules/earn/presentation.rs:255` 返回 category_name_json、introduction_json；`mobile/src/api/earn.ts:58`、`:59` 只取平面名称和分类。locale 仅在缓存键里，不能把这些原文自动翻译。
- 英文界面下，即使后台配置英文贷款名称，手机仍显示默认名称。PC 的 `pc/src/api/loan.ts:225` 已有语言选择函数。
- 修复方向：统一内容选择器，按精确语言、语言族、配置默认和可用内容回退；如产品确实需要同语言不同国家文案，再显式加入国家维度，不能把内容 country 字段当成销售权限。
- 证据：对当前贷款产品适配器提供带英文 name_json 的 mock 返回，结果仍为平面默认名称。

### F6 / P2：手机公告没有按后台目标国家筛选

- `mobile/src/api/news.ts:24` 只传 limit/locale，不传 country_code。
- `src/modules/news/infrastructure.rs:100` 仅在请求带国家时应用国家与 GLOBAL 条件；`src/modules/news/routes.rs:30` 是公开接口，不从登录账户补国家。
- 结果：同语言、不同国家的已发布公告可能混在手机列表中。`mobile/src/api/news.ts:32`、`:45` 也使用顶层标题，而非选择 content_json 内翻译标题。
- 修复方向：确定访客地区与登录后地区来源，列表传递国家并保留全球公告；标题与正文使用同一翻译项。
- 注意：详情接口按设计是公开内容，国家筛选不是保密访问控制。本轮不把它描述成未授权数据泄露；严格地域限制需要另行设计。
- 证据：执行当前 fetchNews，捕获请求确实没有 country_code；服务端过滤与公开性静态核对。

## 应补充的产品能力，不等同于现有故障

1. **稳定国家标识与 KYC 口径**：国家模块接受 2–8 位字母而非权威国家字典（`src/modules/countries/domain.rs:37`）；KYC 下拉保存 country_name（`web/src/admin/actions/KycManagementPage.tsx:224`），后端按文本忽略大小写匹配。建议新写统一稳定国家代码，显示名独立本地化；兼容历史 CN/China/中文名称，不直接重写历史认证记录。注册地区、居住地区、证件签发地区应先定义清楚。
2. **地区充值配置**：`src/modules/quick_recharge/infrastructure.rs:629` 只读 default 单例；`:144` 对应 application 的建单路径使用同一运行配置，币种、网络、限额不是按国家选择。可补国家→支付渠道/法币/限额的配置，后端建单重新验证，订单保存渠道配置版本和必要快照；不能只在前端隐藏渠道。这里仅确认 quick_recharge 模块，不外推所有链上充值能力。
3. **真实多语种扩展**：`src/modules/countries/domain.rs:15`、`web/src/admin/resources/actions/system.tsx:45` 和 Mobile 文案包目前只有中英文。要支持日/韩/越/泰等，需要文案包、后台语言选项、内容翻译覆盖检查、回退和布局验收一起增加，而不是只放开语言字符串。
4. **国家上线检查与生效预览**：建议把国家、语言、KYC、充值与内容配置的有效结果汇总到同一预览，标明不一致和缺失翻译，再发布。现有审计并非不存在，本建议是新增跨配置检查，不是推倒已有治理。
5. **后续专项**：地区产品准入、协议版本/确认记录、展示法币与时区、地区通信渠道，需业务范围确认后分别审查。当前只建议继续评估，未宣称本轮已穷尽全仓所有实现，也不输出各国合规结论。

## 排除的误报

- 注册国家开关已在后端校验：`src/modules/auth/infrastructure.rs:928`，不是仅前端限制。
- KYC 已有按国家证件类型及手持照校验，不是缺少整个功能。
- 新闻编辑保留额外翻译：`web/src/admin/resources/actions/news.tsx:154`、`:223` 使用 additionalContentItems；不能因 UI 仅编辑第一条就断言保存删除所有翻译。新增/编辑更多语言的操作能力仍值得改善。
- 显示语言、国家身份、金融金额精度是不同问题；本轮没有把已完成的数值安全改造重新列为缺失。

## 验证和边界

- `node --test .trellis/tasks/09-26-multi-country-config-review/research/behavior-probes.mjs`：5/5。为 AST 提取当前真实函数后的隔离执行，依赖为 mock，断言记录现状，不是修复验收。
- `node --test --experimental-strip-types mobile/tests/kyc-country-search.test.ts mobile/tests/register-country-search.test.ts mobile/tests/i18n.test.ts mobile/tests/news-rich-text.test.ts`：15/15。
- `npm --prefix web run test -- src/admin/actions/KycManagementPage.test.tsx src/admin/resources/actions/loan.test.tsx`：12/12。已有测试通过不表示上述交叉场景已覆盖。
- 未执行数据库集成、浏览器交互、原生设备、生产配置或真实资金测试，未运行全量发布门禁。

## 实施建议

第一批修 F1/F2/F3，防止内容被改错、KYC 无法完成及国家选项漏项；第二批修 F4/F5/F6，打通后台配置到客户端；第三批设计地区配置中心、支付规则和真正多语种扩展。先保持现有 API 和历史值兼容，再考虑迁移，不自动改历史订单和认证数据。
