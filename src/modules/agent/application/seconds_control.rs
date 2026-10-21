//! 代理门户的秒合约控输控赢入口：身份与可见范围全部来自令牌，不接受客户端指定代理。
//!
//! 订单级与用户级两种控制都先解析当前代理的物化路径子树，再校验目标订单/目标用户是否落在子树内，
//! 校验通过后统一交由秒合约应用层在事务内写控制列与控制流水；本模块不直接写 SQL 也不动资金。
//! 无权与目标不存在统一映射为未找到，避免代理通过响应差异枚举其他团队的用户与订单。

use super::{agent_context, ensure_agent_user_in_scope};
use crate::{
    error::{AppError, AppResult},
    modules::{
        agent::{
            presentation::{AgentSecondsControlRequest, AgentSecondsUserControlResponse},
            service::agent_admin_id_from_subject,
        },
        seconds_contract::{
            application as seconds_contract, infrastructure as seconds_contract_store,
            presentation::{
                SecondsContractControlResponse, SetSecondsContractOrderControlRequest,
                SetSecondsContractUserControlRequest,
            },
            repository::SecondsContractControlActor,
        },
    },
};
use sqlx::{MySql, Pool};

/// 代理对子树内某个用户的单笔秒合约订单设置必胜、必输或取消控制。
/// 先只读定位订单拿归属用户，再做子树校验，最后进入秒合约事务；订单已结算或审核中由下层返回冲突。
/// 订单不存在与订单属于其他代理团队都返回未找到，不泄漏订单归属信息。
pub(crate) async fn set_order_control(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    order_id: u64,
    request: AgentSecondsControlRequest,
) -> AppResult<SecondsContractControlResponse> {
    let agent_admin_id = agent_admin_id_from_subject(subject)?;
    let (pool, scope) = agent_context(mysql, subject).await?;
    let order = seconds_contract::get_admin_order(&pool, order_id).await?;
    ensure_agent_user_in_scope(&pool, &scope, order.user_id).await?;
    seconds_contract::set_order_control(
        Some(&pool),
        order_id,
        SecondsContractControlActor::Agent(agent_admin_id),
        SetSecondsContractOrderControlRequest {
            result: request.result,
            reason: request.reason,
        },
    )
    .await
}

/// 代理对子树内某个用户设置默认控输控赢，作用于该用户后续的秒合约订单。
/// 目标用户必须已在当前代理子树内，否则返回未找到且不写任何控制记录。
/// 用户级设置不会回写存量订单，已开仓订单仍需单独设置订单级控制。
pub(crate) async fn set_user_control(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    user_id: u64,
    request: AgentSecondsControlRequest,
) -> AppResult<SecondsContractControlResponse> {
    let agent_admin_id = agent_admin_id_from_subject(subject)?;
    let (pool, scope) = agent_context(mysql, subject).await?;
    ensure_agent_user_in_scope(&pool, &scope, user_id).await?;
    seconds_contract::set_user_control(
        Some(&pool),
        user_id,
        SecondsContractControlActor::Agent(agent_admin_id),
        SetSecondsContractUserControlRequest {
            result: request.result,
            reason: request.reason,
        },
    )
    .await
}

/// 读取子树内某个用户当前生效的秒合约控制设置；未设置时返回 `active=false` 的空响应。
/// 本接口只读，不加锁、不改订单状态，供代理后台在用户页回显当前控制与操作人。
pub(crate) async fn get_user_control(
    mysql: Option<Pool<MySql>>,
    subject: &str,
    user_id: u64,
) -> AppResult<AgentSecondsUserControlResponse> {
    let (pool, scope) = agent_context(mysql, subject).await?;
    ensure_agent_user_in_scope(&pool, &scope, user_id).await?;
    let snapshot = seconds_contract_store::load_user_control_snapshot(&pool, user_id).await?;
    Ok(match snapshot {
        Some(snapshot) => AgentSecondsUserControlResponse {
            user_id,
            active: true,
            control_result: Some(snapshot.control_result),
            actor_type: Some(snapshot.actor_type),
            actor_id: Some(snapshot.actor_id),
            reason: Some(snapshot.reason).filter(|value| !value.trim().is_empty()),
            control_at: Some(snapshot.control_at),
        },
        None => AgentSecondsUserControlResponse {
            user_id,
            active: false,
            control_result: None,
            actor_type: None,
            actor_id: None,
            reason: None,
            control_at: None,
        },
    })
}

/// 订单级控制必须定位到具体订单，代理传入的订单编号不允许零值。
/// 该检查放在访问数据库之前，非法编号不占用连接也不产生审计噪音。
pub(crate) fn validate_order_id(order_id: u64) -> AppResult<()> {
    if order_id == 0 {
        return Err(AppError::Validation("订单编号无效".to_owned()));
    }
    Ok(())
}
