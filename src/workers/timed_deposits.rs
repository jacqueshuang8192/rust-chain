//! 限时充值只读扫描调度；具体网络启停由后台配置控制，故障不终止常驻循环。
use crate::{error::AppResult, modules::wallet::application::timed_deposits, state::AppState};

/// 每轮重读配置并消费持久游标；节点密钥只传递到加密适配器，错误保留在网络状态中供运营核查。
pub async fn run_loop(state: AppState) -> AppResult<()> {
    let Some(pool) = state.mysql.as_ref() else {
        return Ok(());
    };
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        if let Err(error) =
            timed_deposits::scan_once(pool, state.settings.exposed_credential_encryption_key())
                .await
        {
            tracing::error!(%error,"限时充值扫描轮次失败，将按持久游标重试");
        }
    }
}
