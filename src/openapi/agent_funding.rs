//! 代理资金记录的只读、子树隔离契约。
use crate::{
    error::ErrorResponse,
    modules::agent::presentation::funding::{AgentFundingPage, AgentFundingRecord},
};
use utoipa::OpenApi;

/// 每条记录及总数均限制在令牌代理子树。筛选其他代理树返回空集。
/// 充值来源 quick/chain；快速 paid 统一展示 credited。链上到期、冲正、人工核对沿用用户订单口径。
/// amount 为应付币种数量（快速订单未报价时 null），paid_amount 为已收到币种数量，credited_amount 仅在当前已入账时显示扣费净额。
/// fiat_amount/currency 仅为快速充值法币报价；所有数量为十进制字符串。提现不返回审批、凭据、幂等与网关内部信息。
/// 提现没有商户订单号，order_no 为 null；可按 record_id 查询。详情使用列表中的完整安全投影，无额外越权入口。
#[utoipa::path(get, path="/agent/api/v1/funding/{kind}", tag="agent-portal", security(("bearerAuth"=[])),
    params(
        ("kind"=String,Path,description="deposits | withdrawals"),
        ("team_scope"=Option<String>,Query,description="all（默认）| own | descendants"),
        ("agent_id"=Option<u64>,Query,description="当前子树内分支及其下级，正安全整数"),
        ("user_id"=Option<u64>,Query,description="用户ID，正安全整数"),
        ("record_id"=Option<u64>,Query,description="内部记录ID，正安全整数；充值跨来源可能同ID"),
        ("email"=Option<String>,Query,description="当前邮箱字面包含，最多254字节"),
        ("order_no"=Option<String>,Query,description="充值订单号精确匹配，最多64字节；提现不支持"),
        ("source"=Option<String>,Query,description="充值来源 quick | chain；提现不支持"),
        ("asset_symbol"=Option<String>,Query,description="币种，忽略大小写"),
        ("network"=Option<String>,Query,description="网络代码，忽略大小写"),
        ("status"=Option<String>,Query,description="充值 created|pending|expired|cancelled|review|credited|reversed|failed；提现 pending_review|approved|broadcasting|unknown_broadcast|broadcasted|confirmed|rejected|failed|manual_review"),
        ("created_from"=Option<i64>,Query,description="开始（含），Unix毫秒，0..253402300799999"),
        ("created_to"=Option<i64>,Query,description="结束（不含），必须大于开始"),
        ("limit"=Option<u32>,Query,description="1..100，默认20"),
        ("offset"=Option<u32>,Query,description="0..100000，默认0")
    ), responses(
        (status=200,body=AgentFundingPage),
        (status=400,description="未知字段或非法筛选"),
        (status=401,body=ErrorResponse), (status=403,body=ErrorResponse),
        (status=404,description="未知记录类型"), (status=500,body=ErrorResponse)
    ))]
fn list_records() {}

#[derive(OpenApi)]
#[openapi(
    paths(list_records),
    components(schemas(AgentFundingPage, AgentFundingRecord))
)]
pub(super) struct AgentFundingApiDoc;
