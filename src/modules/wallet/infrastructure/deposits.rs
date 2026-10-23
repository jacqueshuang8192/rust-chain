//! 充币资产、地址池与链上充值事件持久化。
//!
//! 资金不变量：地址分配在事务中唯一；链事件以 network/tx_hash/event_index 幂等，确认入账或重组冲正必须与钱包及流水原子提交。

use super::shared::{
    fetch_admin_page, insert_wallet_ledger_in_tx, insert_wallet_platform_journal_legs_in_tx,
    lock_wallet_balance, update_wallet_balance,
};
use crate::{
    error::{AppError, AppResult},
    modules::wallet::{
        MAX_ASSET_PRECISION_SCALE, WithdrawFeeTier, deposit_net_credit_amount,
        platform_journal::{
            WALLET_DEPOSIT_JOURNAL_CONTEXT, deposit_credit_journal_legs,
            deposit_reversal_journal_legs,
        },
        presentation::{DepositAssetResponse, DepositNetworkResponse, WalletDepositEventResponse},
        truncate_amount_to_asset_precision,
    },
};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{MySql, Pool, QueryBuilder, Transaction, types::Json as SqlxJson};

#[derive(Debug, sqlx::FromRow)]
struct DepositNetworkRow {
    network: String,
    display_name: String,
    address_group_code: String,
    address_group_name: Option<String>,
    asset_symbols: SqlxJson<Vec<String>>,
}

#[derive(Debug, sqlx::FromRow)]
struct DepositAssetRow {
    symbol: String,
    name: String,
    logo_url: Option<String>,
    precision_scale: i32,
    deposit_enabled: bool,
    withdraw_enabled: bool,
    min_deposit_amount: BigDecimal,
    deposit_fee: BigDecimal,
    withdraw_fee: BigDecimal,
    withdraw_fee_tiers: SqlxJson<Vec<WithdrawFeeTier>>,
}

/// 列出状态启用且开放充值的资产及其精度、最小额和费率配置，按资产代码升序返回。
/// 只筛资产自身的启用与充值开关，不检查是否存在可用网络或地址库存，前端仍可能选到无地址可分配的资产。
/// 响应同时带出提现开关与提现费用，是资产维度的完整配置快照，本查询不触碰任何余额或地址池。
pub(crate) async fn list_deposit_assets(
    pool: &Pool<MySql>,
) -> AppResult<Vec<DepositAssetResponse>> {
    let rows = sqlx::query_as::<_, DepositAssetRow>(&deposit_assets_sql(true))
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(deposit_asset_response).collect())
}

/// 列出状态启用且开放提现的资产及固定手续费、阶梯费率与精度配置，按资产代码升序返回。
/// 与充值清单共用同一套字段，仅把启用开关换成提现开关，因此同一资产可能只出现在其中一侧清单。
/// 阶梯费率此处按存量原样返回、未做重叠校验，真正下单时仍以服务端规范化后的规则重新计费。
pub(crate) async fn list_withdraw_assets(
    pool: &Pool<MySql>,
) -> AppResult<Vec<DepositAssetResponse>> {
    let rows = sqlx::query_as::<_, DepositAssetRow>(&deposit_assets_sql(false))
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(deposit_asset_response).collect())
}

/// 按可选资产过滤启用的充值网络，并保留地址组共享配置。
/// 资产过滤同时接受两类网络：未配置资产白名单的通用网络，以及白名单中显式包含该资产的网络。
/// 结果按配置排序值再按主键升序，保证同一入参下网络顺序稳定，前端可直接用于默认选中项。
/// 查询只读网络元数据，不分配地址，也不预留或修改地址池库存。
pub(crate) async fn list_active_deposit_networks(
    pool: &Pool<MySql>,
    asset_symbol: Option<&str>,
) -> AppResult<Vec<DepositNetworkResponse>> {
    let mut builder = QueryBuilder::<MySql>::new(deposit_networks_sql());
    if let Some(symbol) = asset_symbol {
        builder.push(
            " AND (asset_symbols_json IS NULL OR JSON_CONTAINS(asset_symbols_json, JSON_QUOTE(",
        );
        builder.push_bind(symbol);
        builder.push(")))");
    }
    builder.push(" ORDER BY sort_order ASC, id ASC");
    let rows = builder
        .build_query_as::<DepositNetworkRow>()
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(deposit_network_response).collect())
}

#[derive(Debug)]
pub struct NewWalletChainEventDeadLetter<'a> {
    pub gateway_id: u64,
    pub network: &'a str,
    pub event_kind: &'a str,
    pub dedup_key: String,
    pub request_id: Option<String>,
    pub tx_hash: Option<String>,
    pub event_index: Option<u32>,
    pub payload_json: String,
    pub failure_reason: String,
}

/// 记录无法处理的链事件，保留原始载荷和最后一次失败原因供人工追查。
/// 以去重键为唯一身份做插入或更新，同一事件反复失败只会覆盖载荷与失败原因，不会堆积重复死信行。
/// 覆盖语义意味着历史失败原因会被最新一次替换，需要完整失败序列的场景应另行采集日志。
/// 写入走连接池独立执行、不参与调用方事务，因此死信记录不会随业务回滚而消失。
/// 死信写入不代表资金成功入账，也不得替代原链事件的幂等身份判断。
pub async fn insert_wallet_chain_event_dead_letter(
    pool: &Pool<MySql>,
    record: &NewWalletChainEventDeadLetter<'_>,
) -> AppResult<()> {
    sqlx::query(
        r#"INSERT INTO wallet_chain_event_dead_letters
              (gateway_id, network, event_kind, dedup_key, request_id, tx_hash, event_index,
               payload_json, failure_reason)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
           ON DUPLICATE KEY UPDATE
             payload_json = VALUES(payload_json),
             failure_reason = VALUES(failure_reason)"#,
    )
    .bind(record.gateway_id)
    .bind(record.network)
    .bind(record.event_kind)
    .bind(&record.dedup_key)
    .bind(&record.request_id)
    .bind(&record.tx_hash)
    .bind(record.event_index)
    .bind(&record.payload_json)
    .bind(&record.failure_reason)
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, sqlx::FromRow)]
pub struct WalletChainEventDeadLetterRecord {
    pub id: u64,
    pub gateway_id: u64,
    pub network: String,
    pub event_kind: String,
    pub request_id: Option<String>,
    pub tx_hash: Option<String>,
    pub event_index: Option<u32>,
    pub payload_json: SqlxJson<serde_json::Value>,
    pub failure_reason: String,
    pub created_at: DateTime<Utc>,
}

/// 按可选网络筛选读取链事件死信，结果以主键倒序返回最近失败的记录。
/// 返回条数会被钳制在一到五百之间，防止调用方传零或超大值把整表拉进内存。
/// 返回载荷是原始网关字段的结构化副本，可用于人工重放，但字段可信度不高于当初的网关响应。
/// 查询只读、不修改任何处理标记，也不能据返回结果直接补写钱包余额或充值流水。
pub async fn list_wallet_chain_event_dead_letters(
    pool: &Pool<MySql>,
    network: Option<&str>,
    limit: u32,
) -> AppResult<Vec<WalletChainEventDeadLetterRecord>> {
    let mut builder = QueryBuilder::<MySql>::new(
        r#"SELECT id, gateway_id, network, event_kind, request_id, tx_hash, event_index,
                  payload_json, failure_reason, created_at
           FROM wallet_chain_event_dead_letters"#,
    );
    if let Some(network) = network {
        builder.push(" WHERE network = ");
        builder.push_bind(network);
    }
    builder.push(" ORDER BY id DESC LIMIT ");
    builder.push_bind(i64::from(limit.clamp(1, 500)));
    builder
        .build_query_as::<WalletChainEventDeadLetterRecord>()
        .fetch_all(pool)
        .await
        .map_err(AppError::from)
}

/// 锁定已入账事件后处理链重组冲正；已 reversed 时直接返回，其他非 credited 状态冲突。
/// available 足额时扣回当时净入账额，frozen/locked 不变，并写一条 `deposit_reorg_reverse` 负向 available 流水后标记 reversed。
/// 冲正金额恒等于毛额减去快照手续费后的净额，扣减后的 available 按 18 位定点写回，流水以事件编号为业务引用，与入账条目共用同一身份。
/// 锁顺序与入账一致：先按事件主键取排他锁，再锁钱包账户行，因此正向入账与反向冲正不会互相形成环路等待。
/// available 不足时不扣任何余额、不写冲正流水，而是提交 manual_review 与失败原因，保留人工处置事实。
/// 状态更新均带原状态条件，配合事件行锁保证并发重放中最多一次生效；事件、余额和流水由该函数自有事务提交。
/// 重复调用不会二次扣款，任一 SQL 失败整体回滚，不会留下余额已扣但状态未改的中间态。
pub(crate) async fn reverse_deposit_event(
    pool: &Pool<MySql>,
    deposit_id: u64,
    reason: &str,
) -> AppResult<WalletDepositEventResponse> {
    let mut tx = pool.begin().await?;
    let event = load_deposit_event_by_id_for_update(&mut tx, deposit_id).await?;
    if event.status == "reversed" {
        tx.commit().await?;
        return Ok(event);
    }
    if event.status != "credited" {
        return Err(AppError::Conflict(format!(
            "deposit cannot be reversed from status {}",
            event.status
        )));
    }
    let credit_amount = deposit_credit_amount(&event)?;
    let wallet = lock_wallet_balance(&mut tx, event.user_id, event.asset_id).await?;
    if wallet.available < credit_amount {
        sqlx::query(
            r#"UPDATE wallet_deposit_events
               SET status = 'manual_review', failure_reason = ?
               WHERE id = ? AND status = 'credited'"#,
        )
        .bind(reason)
        .bind(event.id)
        .execute(&mut *tx)
        .await?;
        let event = load_deposit_event_by_id_in_tx(&mut tx, event.id).await?;
        tx.commit().await?;
        return Ok(event);
    }
    let available_after = wallet.available.clone() - credit_amount.clone();
    update_wallet_balance(
        &mut tx,
        event.user_id,
        event.asset_id,
        &available_after,
        &wallet.frozen,
        &wallet.locked,
    )
    .await?;
    insert_wallet_ledger_in_tx(
        &mut tx,
        event.user_id,
        event.asset_id,
        "deposit_reorg_reverse",
        &(-credit_amount.clone()),
        "available",
        &available_after,
        &available_after,
        &wallet.frozen,
        &wallet.locked,
        "wallet_deposit_event",
        &event.id.to_string(),
    )
    .await?;
    insert_wallet_platform_journal_legs_in_tx(
        &mut tx,
        WALLET_DEPOSIT_JOURNAL_CONTEXT,
        &format!("wallet_deposit:{}:reverse", event.id),
        event.asset_id,
        WALLET_DEPOSIT_JOURNAL_CONTEXT,
        event.id,
        &deposit_reversal_journal_legs(&deposit_journal_gross_amount(&event), &credit_amount),
    )
    .await?;
    sqlx::query(
        r#"UPDATE wallet_deposit_events
           SET status = 'reversed', failure_reason = ?, reversed_at = CURRENT_TIMESTAMP(6)
           WHERE id = ? AND status = 'credited'"#,
    )
    .bind(reason)
    .bind(event.id)
    .execute(&mut *tx)
    .await?;
    let event = load_deposit_event_by_id_in_tx(&mut tx, event.id).await?;
    tx.commit().await?;
    Ok(event)
}

/// 后台充值事件列表：行查询与 COUNT 共用同一组谓词，总数才会跟随当前筛选。
/// 按后台筛选条件查询充值链事件并使用同一谓词计算总数。
/// 该只读入口不触发入账、冲正或游标推进，分页结果应与总数保持一致。
pub(crate) async fn list_deposit_events(
    pool: &Pool<MySql>,
    user_id: Option<u64>,
    limit: u32,
    offset: u32,
) -> AppResult<(Vec<WalletDepositEventResponse>, i64)> {
    let mut rows = QueryBuilder::<MySql>::new(wallet_deposit_select_sql());
    let mut total = QueryBuilder::<MySql>::new("SELECT COUNT(*) FROM wallet_deposit_events events");
    for builder in [&mut rows, &mut total] {
        builder.push(" WHERE 1 = 1");
        if let Some(user_id) = user_id {
            builder.push(" AND events.user_id = ");
            builder.push_bind(user_id);
        }
    }

    fetch_admin_page(
        pool,
        rows,
        total,
        " ORDER BY events.id DESC",
        limit.clamp(1, 200),
        offset,
    )
    .await
}

/// 生成资产配置清单 SQL，按入参在充值开关和提现开关之间二选一作为过滤列。
/// 开关列名由代码内布尔量决定而非外部输入拼接，因此不存在注入风险；两种模式的选择列与排序完全一致。
/// 阶梯费率列在为空时兜底成空 JSON 数组，避免反序列化阶段因空值报错而使整张清单不可用。
fn deposit_assets_sql(deposit_enabled: bool) -> String {
    let flag = if deposit_enabled {
        "deposit_enabled"
    } else {
        "withdraw_enabled"
    };
    format!(
        r#"SELECT symbol,
                  name,
                  logo_url,
                  precision_scale,
                  deposit_enabled,
                  withdraw_enabled,
                  min_deposit_amount,
                  deposit_fee,
                  withdraw_fee,
                  COALESCE(withdraw_fee_tiers_json, JSON_ARRAY()) AS withdraw_fee_tiers
           FROM assets
           WHERE status = 'active' AND {flag} = TRUE
           ORDER BY symbol ASC"#
    )
}

/// 返回启用充值网络查询的固定前缀，条件停在状态过滤处，供调用方继续追加资产白名单与排序。
/// 资产白名单列为空时兜底成空 JSON 数组，使无白名单的通用网络也能安全反序列化。
fn deposit_networks_sql() -> &'static str {
    r#"SELECT network,
              display_name,
              address_group_code,
              address_group_name,
              COALESCE(asset_symbols_json, JSON_ARRAY()) AS asset_symbols
       FROM deposit_network_configs
       WHERE status = 'active'"#
}

/// 返回充值链事件的统一选择列与来源表，供列表分页、外部键锁定和主键回读三条路径复用同一投影。
/// 共用投影保证幂等判定所依赖的地址、金额、备注、确认数和状态字段在各入口取值口径完全一致。
fn wallet_deposit_select_sql() -> &'static str {
    r#"SELECT events.id, events.user_id, events.asset_id, events.asset_symbol,
              events.network, events.address, events.memo, events.tx_hash, events.event_index,
              events.amount, events.fee_amount, events.block_height, events.confirmations,
              events.required_confirmations, events.status, events.failure_reason,
              events.credited_at, events.reversed_at, events.created_at
       FROM wallet_deposit_events events"#
}

/// 按事件主键加排他锁读取充值事件，供后台冲正等以内部编号发起的操作串行化。
/// 与按外部键加锁的入口作用相同但定位方式不同，两者都必须先于钱包账户锁获取以维持同向锁序。
async fn load_deposit_event_by_id_for_update(
    tx: &mut Transaction<'_, MySql>,
    event_id: u64,
) -> AppResult<WalletDepositEventResponse> {
    sqlx::query_as::<_, WalletDepositEventResponse>(&format!(
        "{} WHERE events.id = ? LIMIT 1 FOR UPDATE",
        wallet_deposit_select_sql()
    ))
    .bind(event_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)
}

/// 在事务内按主键回读充值事件的最新状态，用于把入账或冲正后的结果返回给调用方。
/// 该读取不加锁，因为调用方此前已持有同一行的排他锁，重复加锁只会增加等待而无额外保护。
async fn load_deposit_event_by_id_in_tx(
    tx: &mut Transaction<'_, MySql>,
    event_id: u64,
) -> AppResult<WalletDepositEventResponse> {
    sqlx::query_as::<_, WalletDepositEventResponse>(&format!(
        "{} WHERE events.id = ? LIMIT 1",
        wallet_deposit_select_sql()
    ))
    .bind(event_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)
}

/// 在调用方事务中完成一笔充值的实际入账：锁钱包、把毛额减去快照手续费后的净额记入 available、写正向流水并把事件推进为已入账。
/// 资金只进 available，frozen 与 locked 原值回写；入账后的可用余额按 18 位定点计算，避免不同链精度污染账本。
/// 流水变更类型固定为 deposit_confirm，业务引用指向充值事件编号，因此重复入账可由引用维度直接甄别。
/// 状态更新带上原状态为已观测的条件，受影响行数不为一即判定并发抢先，返回冲突让整个事务回滚。
/// 该函数不做确认数是否达标的判断，也不校验金额精度，前置条件全部由调用方在同一事务内完成。
async fn credit_deposit_event_in_tx(
    tx: &mut Transaction<'_, MySql>,
    event: &WalletDepositEventResponse,
) -> AppResult<()> {
    let credit_amount = deposit_credit_amount(event)?;
    let wallet = lock_wallet_balance(tx, event.user_id, event.asset_id).await?;
    let available_after = wallet.available.clone() + credit_amount.clone();
    update_wallet_balance(
        tx,
        event.user_id,
        event.asset_id,
        &available_after,
        &wallet.frozen,
        &wallet.locked,
    )
    .await?;
    insert_wallet_ledger_in_tx(
        tx,
        event.user_id,
        event.asset_id,
        "deposit_confirm",
        &credit_amount,
        "available",
        &available_after,
        &available_after,
        &wallet.frozen,
        &wallet.locked,
        "wallet_deposit_event",
        &event.id.to_string(),
    )
    .await?;
    insert_wallet_platform_journal_legs_in_tx(
        tx,
        WALLET_DEPOSIT_JOURNAL_CONTEXT,
        &format!("wallet_deposit:{}:credit", event.id),
        event.asset_id,
        WALLET_DEPOSIT_JOURNAL_CONTEXT,
        event.id,
        &deposit_credit_journal_legs(&deposit_journal_gross_amount(event), &credit_amount),
    )
    .await?;
    let update = sqlx::query(
        r#"UPDATE wallet_deposit_events
           SET status = 'credited', credited_at = CURRENT_TIMESTAMP(6)
           WHERE id = ? AND status = 'observed'"#,
    )
    .bind(event.id)
    .execute(&mut **tx)
    .await?;
    if update.rows_affected() != 1 {
        return Err(AppError::Conflict(
            "deposit event status changed concurrently".to_owned(),
        ));
    }
    Ok(())
}

/// 限时订单已在同事务锁定归属后调用本入口；先登记独立链事件，再复用钱包、流水和平台分录写入。
/// 不依赖地址的当前所有人；外部事件唯一键冲突整体回滚，禁止覆盖永久充值或其他订单已有入账。
pub(super) async fn credit_matched_order_in_tx(
    tx: &mut Transaction<'_, MySql>,
    event: &WalletDepositEventResponse,
) -> AppResult<u64> {
    if event.confirmations < event.required_confirmations {
        return Err(AppError::Conflict(
            "deposit confirmations are insufficient".to_owned(),
        ));
    }
    crate::numeric::ensure_amount_storage(&event.amount, "deposit amount")?;
    deposit_credit_amount(event)?;
    let id = sqlx::query(
        "INSERT INTO wallet_deposit_events (user_id,asset_id,asset_symbol,network,address,memo,tx_hash,event_index,amount,fee_amount,block_height,confirmations,required_confirmations,status) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,'observed')",
    )
    .bind(event.user_id).bind(event.asset_id).bind(&event.asset_symbol).bind(&event.network)
    .bind(&event.address).bind(&event.memo).bind(&event.tx_hash).bind(event.event_index)
    .bind(&event.amount).bind(&event.fee_amount).bind(event.block_height)
    .bind(event.confirmations).bind(event.required_confirmations)
    .execute(&mut **tx).await?.last_insert_id();
    let mut saved = event.clone();
    saved.id = id;
    credit_deposit_event_in_tx(tx, &saved).await?;
    Ok(id)
}

/// 用事件快照上的毛额和手续费算出实际入账净额；手续费为负或净额非正则拒绝。
fn deposit_credit_amount(event: &WalletDepositEventResponse) -> AppResult<BigDecimal> {
    deposit_net_credit_amount(&event.amount, &event.fee_amount, MAX_ASSET_PRECISION_SCALE)
        .map_err(AppError::Validation)
}

/// 平台分录使用的托管毛额：与钱包入账同一口径，把链上原始金额定点化到账本最大精度。
/// 它减去净额正好等于入账时使用的快照手续费，因此平台分录与用户入账描述的是同一笔钱。
/// 纯换算，不读数据库也不写钱包；调用方须与净额同源，避免两条腿落在不同精度上。
fn deposit_journal_gross_amount(event: &WalletDepositEventResponse) -> BigDecimal {
    truncate_amount_to_asset_precision(&event.amount, MAX_ASSET_PRECISION_SCALE)
}

/// 把资产配置行搬运为充提资产响应项，同时带出充值与提现两侧开关供前端判断可用动作。
/// 精度、最小充值额与两类费用按定点原值输出，阶梯费率从 JSON 列展开为结构体数组，本转换不校验阶梯是否重叠。
fn deposit_asset_response(row: DepositAssetRow) -> DepositAssetResponse {
    DepositAssetResponse {
        symbol: row.symbol,
        name: row.name,
        logo_url: row.logo_url,
        precision_scale: row.precision_scale,
        deposit_enabled: row.deposit_enabled,
        withdraw_enabled: row.withdraw_enabled,
        min_deposit_amount: row.min_deposit_amount,
        deposit_fee: row.deposit_fee,
        withdraw_fee: row.withdraw_fee,
        withdraw_fee_tiers: row.withdraw_fee_tiers.0,
    }
}

/// 把充值网络配置行搬运为网络响应项，附带地址组代码与组名以便前端说明地址复用范围。
/// 资产白名单从 JSON 列展开为字符串数组，空数组表示该网络对所有资产通用而非不支持任何资产。
fn deposit_network_response(row: DepositNetworkRow) -> DepositNetworkResponse {
    DepositNetworkResponse {
        network: row.network,
        display_name: row.display_name,
        address_group_code: row.address_group_code,
        address_group_name: row.address_group_name,
        asset_symbols: row.asset_symbols.0,
    }
}
