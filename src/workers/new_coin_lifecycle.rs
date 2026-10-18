//! 新版发行阶段调度；不派发、不释放持仓、不卖币，排程与失败退避全部由数据库保存。
use crate::{error::AppResult, state::AppState};
/// 有界执行到期阶段；与手动操作共用项目锁和行情写入锁，多个实例重复运行仍只有一次实际事件。
pub async fn run_once(state: &AppState) -> AppResult<u32> {
    match &state.mysql {
        Some(pool) => crate::modules::new_coin::application::issuance::advance_due(pool).await,
        None => Ok(0),
    }
}
/// 每十秒扫描已批准项目；单项失败有持久化退避，数据库不可用时记录错误并等待下一轮。
pub async fn run_loop(state: AppState) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        if let Err(error) = run_once(&state).await {
            tracing::error!(error=%error,"新币阶段调度失败");
        }
    }
}
