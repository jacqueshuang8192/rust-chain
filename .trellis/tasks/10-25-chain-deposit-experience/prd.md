# 链上充值核对、订单记录与页面体验

## Goal
修复管理员核对操作，明确用户提前取消规则，将充值订单记录拆为独立页面，并按指定 design-taste-frontend 的适用设计原则重做链上充值界面。

## Known requirements
- 管理员反馈核对按钮无效果，需复现并修复实际操作链路。
- 用户充值订单记录应独立页面。
- 页面重做排版、状态及操作层级，保留真实金额、网络、地址、30分钟截止及权限合同。
- design-taste-frontend 明确不适用于 dashboard/app screens；本任务是交易应用界面，采用其排版/配色/真实截图复查原则，不套用营销页图片数量或入场动画要求。

## Confirmed scope
- 用户确认：增加未付款订单主动取消，之后迟到款转人工核对。
- 用户确认：仅手机端样式重做与订单记录独立页；后台仅修复核对按钮，不拆分后台页面，不重做PC。
- 取消保留不可变创建/原到期时间，记录取消时间；在网络锁内阻止已到账或待核对订单被取消，取消后释放占用但保留歧义历史。
- PC及代理只做识别新增 cancelled 状态所必需的兼容，不改变布局。

## Acceptance criteria
- [x] 核对入口具备明确结果和异常反馈，回归测试覆盖真实交互。
- [x] 按确认后的提前取消规则实现及测试资金/归属边界。
- [x] 充值订单记录独立路由，可查看与返回当前订单，保持分页和权限。
- [x] 桌面与窄屏布局验收，最贴近变更的测试及质量门禁完成。

## Out of scope
不增加网络/扫描协议，不变更快速充值、提现或无关交易功能；推送按用户明确授权执行；不部署。

## Delivery status
Implemented and verified locally. User explicitly authorized commit and push on 2026-10-25. Delivery includes the work commit, task archive and session journal on origin/main. See delivery.md for measured budgets, validation limits and exact file list.
