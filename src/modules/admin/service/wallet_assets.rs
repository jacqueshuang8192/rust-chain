//! 资产定义与提现网络配置的纯业务规则层。
//!
//! 本文件全部是不触库、不加锁的纯函数，职责分三类：把请求字段规范化成稳定的落库形态（符号大写、网络别名归一、
//! 地址组代码大写）、校验单字段与跨字段约束（精度区间、非负金额、批次上限、资产是否在网络白名单内）、
//! 以及生成审计前后值快照。所有唯一性、引用完整性和并发安全都不在这里判断，
//! 而是由 application 层的事务加锁与数据库约束负责，因此这里通过校验并不意味着写入一定成功。

use super::*;

/// 交易对数量必须能由基准资产无损结算，最小成交额必须符合计价资产精度；坏资产元数据直接失败。
pub(crate) fn validate_trading_pair_asset_precisions(
    base_precision: i32,
    quote_precision: i32,
    qty_precision: i32,
    min_order_value: &BigDecimal,
) -> AppResult<()> {
    if !(0..=18).contains(&base_precision) || !(0..=18).contains(&quote_precision) {
        return Err(AppError::Internal(
            "invalid trading pair asset precision".to_owned(),
        ));
    }
    if !(0..=base_precision).contains(&qty_precision) {
        return Err(AppError::Validation(
            "qty_precision exceeds base asset precision".to_owned(),
        ));
    }
    crate::numeric::ensure_amount_storage(min_order_value, "min_order_value")?;
    if !crate::modules::wallet::amount_fits_asset_precision(min_order_value, quote_precision) {
        return Err(AppError::Validation(
            "min_order_value exceeds quote asset precision".to_owned(),
        ));
    }
    Ok(())
}

/// 校验新资产符号、名称、精度、类型、状态、充提限额费率及阶梯提现费配置。
/// 不查询 symbol 唯一性或关联交易对；这些并发约束由创建事务与数据库唯一键保证。
/// 资产类型与状态在创建请求中是可选项，仅在显式提供时才校验，缺省值由应用层补齐而非在此处填充。
/// 三项金额与阶梯费同样是可选项，提供时按非负和阶梯不重叠的规则判定，未提供则留给应用层落默认零值。
pub(crate) fn validate_create_asset_request(request: &CreateAssetRequest) -> AppResult<()> {
    normalize_asset_symbol(&request.symbol)?;
    validate_asset_name(&request.name)?;
    validate_asset_precision(request.precision_scale)?;
    validate_optional_asset_amount(request.min_deposit_amount.as_ref(), "min_deposit_amount")?;
    validate_optional_asset_amount(request.deposit_fee.as_ref(), "deposit_fee")?;
    validate_optional_asset_amount(request.withdraw_fee.as_ref(), "withdraw_fee")?;
    validate_optional_withdraw_fee_tiers(request.withdraw_fee_tiers.as_deref())?;
    if let Some(asset_type) = request.asset_type.as_deref() {
        validate_asset_type(asset_type)?;
    }
    if let Some(status) = request.status.as_deref() {
        validate_asset_status(status)?;
    }
    Ok(())
}

/// 校验资产更新中的名称、精度、类型、状态、充提限额费率、阶梯费及审计原因。
/// 这里只校验目标配置快照；变更精度或删除资产的引用安全由应用事务锁定后确认。
pub(crate) fn validate_update_asset_request(request: &UpdateAssetRequest) -> AppResult<()> {
    validate_asset_name(&request.name)?;
    validate_asset_precision(request.precision_scale)?;
    validate_asset_type(&request.asset_type)?;
    validate_asset_status(&request.status)?;
    validate_optional_asset_amount(request.min_deposit_amount.as_ref(), "min_deposit_amount")?;
    validate_optional_asset_amount(request.deposit_fee.as_ref(), "deposit_fee")?;
    validate_optional_asset_amount(request.withdraw_fee.as_ref(), "withdraw_fee")?;
    validate_optional_withdraw_fee_tiers(request.withdraw_fee_tiers.as_deref())?;
    required_admin_audit_reason(request.reason.clone())?;
    Ok(())
}

/// 校验最小充值额、充值费和固定提现费均为非负值；不执行资产精度截断或阶梯费匹配。
/// 与请求级校验不同，这里接收的是已合并旧值后的最终三项金额，因此更新时只改其中一项也会整体重新判定。
/// 三者按固定顺序依次检查并在首个非法项处返回，错误文案带字段名以便定位。
pub(crate) fn validate_asset_fee_settings(
    min_deposit_amount: &BigDecimal,
    deposit_fee: &BigDecimal,
    withdraw_fee: &BigDecimal,
) -> AppResult<()> {
    validate_asset_amount(min_deposit_amount, "min_deposit_amount")?;
    validate_asset_amount(deposit_fee, "deposit_fee")?;
    validate_asset_amount(withdraw_fee, "withdraw_fee")
}

/// 规范化资产提现阶梯费配置并返回按共享钱包规则校验后的阶梯集合。
/// 金额边界、费率形状或阶梯重叠等错误转换为后台校验错误；函数不读取资产精度或保存配置。
pub(crate) fn normalize_asset_withdraw_fee_tiers(
    tiers: Vec<WithdrawFeeTier>,
) -> AppResult<Vec<WithdrawFeeTier>> {
    normalize_withdraw_fee_tiers(tiers).map_err(AppError::Validation)
}

/// 去除资产名称首尾空白并限制数据库列长度；空名称返回校验错误。
/// 长度上限 128 按字节统计而非字符，因此中文名称的实际可用字数约为三分之一，纯空白等同于缺失。
pub(crate) fn validate_asset_name(value: &str) -> AppResult<String> {
    let Some(name) = optional_string(Some(value.to_owned())) else {
        return Err(AppError::Validation("asset name is required".to_owned()));
    };
    if name.len() > 128 {
        return Err(AppError::Validation(
            "asset name must be at most 128 characters".to_owned(),
        ));
    }
    Ok(name)
}

/// 限制资产小数精度为 0..=18；本函数不重算已有余额或流水。
/// 上限 18 对齐数据库十进制列的可用标度，负数同样被拒；调低精度属于合法输入，
/// 但既有余额与流水不会被截断或重算，需要另行评估对账影响。
pub(crate) fn validate_asset_precision(value: i32) -> AppResult<()> {
    if !(0..=18).contains(&value) {
        return Err(AppError::Validation(
            "asset precision_scale must be between 0 and 18".to_owned(),
        ));
    }
    Ok(())
}

/// 将资产符号去除空白并转为大写，生成最多 32 字节的稳定数据库代码。
/// 符号只允许 ASCII 字母数字；空白或非法格式返回校验错误，唯一性由资产写事务确认。
pub(crate) fn normalize_asset_symbol(value: &str) -> AppResult<String> {
    let Some(symbol) = optional_string(Some(value.to_owned())) else {
        return Err(AppError::Validation("asset symbol is required".to_owned()));
    };
    if symbol.len() > 32 || !symbol.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        return Err(AppError::Validation(
            "asset symbol format is invalid".to_owned(),
        ));
    }
    Ok(symbol.to_ascii_uppercase())
}

/// 规范化资产类型为后台支持的法币、加密资产等稳定代码；未知类型拒绝持久化。
/// 只接受 coin、fiat、stablecoin、platform 四个取值，比对在去空后进行且区分大小写，不做大小写归一。
pub(crate) fn validate_asset_type(value: &str) -> AppResult<String> {
    let Some(asset_type) = optional_string(Some(value.to_owned())) else {
        return Err(AppError::Validation("asset_type is required".to_owned()));
    };
    match asset_type.as_str() {
        "coin" | "fiat" | "stablecoin" | "platform" => Ok(asset_type),
        _ => Err(AppError::Validation("unsupported asset_type".to_owned())),
    }
}

/// 规范化资产启停状态；不在此处判断是否仍被交易对、钱包或产品引用。
/// 仅接受 active 与 disabled 两值。删除资产要求先切到 disabled，但引用检查发生在删除事务里而不是这里。
pub(crate) fn validate_asset_status(value: &str) -> AppResult<String> {
    let Some(status) = optional_string(Some(value.to_owned())) else {
        return Err(AppError::Validation("status is required".to_owned()));
    };
    match status.as_str() {
        "active" | "disabled" => Ok(status),
        _ => Err(AppError::Validation("unsupported asset status".to_owned())),
    }
}

/// 将资产符号、精度、充提与杠杆转入规则、Logo、阶梯费和状态映射为稳定审计 JSON。
/// 快照还包含资产类型和创建时间，但不含钱包余额；应用层在资产配置事务中保存前后值。
/// 阶梯提现费以数组原样展开而非摘要，便于在审计里直接看清各档区间与费率的增删改。
/// 创建、更新与删除三类操作共用这份结构，删除时只写 before 而 after 为空。
pub(crate) fn asset_audit_json(asset: &AdminAssetResponse) -> Value {
    json!({
        "id": asset.id,
        "symbol": asset.symbol,
        "name": asset.name,
        "logo_url": asset.logo_url,
        "precision_scale": asset.precision_scale,
        "asset_type": asset.asset_type,
        "status": asset.status,
        "deposit_enabled": asset.deposit_enabled,
        "withdraw_enabled": asset.withdraw_enabled,
        "margin_transfer_enabled": asset.margin_transfer_enabled,
        "min_deposit_amount": asset.min_deposit_amount,
        "deposit_fee": asset.deposit_fee,
        "withdraw_fee": asset.withdraw_fee,
        "withdraw_fee_tiers": asset.withdraw_fee_tiers.0.clone(),
        "created_at": asset.created_at.timestamp_millis(),
    })
}

/// 将充值网络别名规范为 `eth`、`base`、`tron`、`btc` 或 `solana` 五种稳定小写代码。
/// 支持 Ethereum/ERC20、Tron/TRC20、Bitcoin 和 Solana 常见别名；未知网络返回参数错误。
/// 匹配前先去空白再统一转小写，因此大小写混写的别名同样能被识别。
/// 归一后的代码是网络配置与地址池共用的连接键，别名扩展只应在此处集中增加，避免两侧口径漂移。
pub(crate) fn normalize_deposit_network(value: &str) -> AppResult<String> {
    let Some(network) = optional_string(Some(value.to_owned())) else {
        return Err(AppError::Validation("network is required".to_owned()));
    };
    match network.to_ascii_lowercase().as_str() {
        "eth" | "ethereum" | "erc20" => Ok("eth".to_owned()),
        "base" => Ok("base".to_owned()),
        "tron" | "trx" | "trc20" => Ok("tron".to_owned()),
        "btc" | "bitcoin" => Ok("btc".to_owned()),
        "sol" | "solana" => Ok("solana".to_owned()),
        _ => Err(AppError::Validation(
            "unsupported deposit network".to_owned(),
        )),
    }
}

/// 去除充值网络展示名首尾空白并限制 64 个字符，供前端网络选择列表显示。
/// 与网络代码不同，展示名不参与任何匹配逻辑、不做大小写归一，纯粹面向界面呈现，因此按字符数而非字节数限长。
pub(crate) fn validate_deposit_network_display_name(value: &str) -> AppResult<String> {
    let Some(display_name) = optional_string(Some(value.to_owned())) else {
        return Err(AppError::Validation("display_name is required".to_owned()));
    };
    if display_name.chars().count() > 64 {
        return Err(AppError::Validation("display_name is too long".to_owned()));
    }
    Ok(display_name)
}

/// 规范化充值地址组代码并限制字符集合和长度，使地址池分配查询可稳定匹配。
/// 字符集限定为 ASCII 字母数字与下划线、横线，长度上限 64 字符，最终统一转为大写后返回。
/// 因为返回值会同时用于网络配置与地址池两侧的比对，大小写归一是二者能匹配上的前提，
/// 该函数也被复用来校验从数据库读回的配置值，以便及早发现历史脏数据。
pub(crate) fn validate_address_group_code(value: &str) -> AppResult<String> {
    let Some(code) = optional_string(Some(value.to_owned())) else {
        return Err(AppError::Validation(
            "address_group_code is required".to_owned(),
        ));
    };
    if code.chars().count() > 64
        || !code
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err(AppError::Validation(
            "address_group_code format is invalid".to_owned(),
        ));
    }
    Ok(code.to_ascii_uppercase())
}

/// 规范化提现网络配置状态，仅允许提现准入明确支持的生命周期代码。
/// 取值同样只有 active 与 disabled，但语义与资产状态不同：停用的网络会让该网络下所有地址无法通过准入校验。
pub(crate) fn validate_deposit_network_config_status(value: &str) -> AppResult<String> {
    let Some(status) = optional_string(Some(value.to_owned())) else {
        return Err(AppError::Validation("status is required".to_owned()));
    };
    match status.as_str() {
        "active" | "disabled" => Ok(status),
        _ => Err(AppError::Validation(
            "unsupported deposit network config status".to_owned(),
        )),
    }
}

/// 去除可选文本两端空白，将空串转为空值，并按调用方字段名执行字符长度上限校验。
/// 超长值返回带字段名的校验错误；函数不解释字段业务含义，也不持久化结果。
pub(crate) fn validate_optional_length(
    value: Option<String>,
    field: &str,
    max_len: usize,
) -> AppResult<Option<String>> {
    let Some(value) = optional_string(value) else {
        return Ok(None);
    };
    if value.chars().count() > max_len {
        return Err(AppError::Validation(format!("{field} is too long")));
    }
    Ok(Some(value))
}

/// 合并充值配置的单资产兼容字段和资产数组，返回去空、转大写且保持首次出现顺序的符号集合。
/// 优先使用非空数组，仅在数组结果为空时回退单值；重复项被忽略，超过 50 项或任一符号非法时返回校验错误。
pub(crate) fn normalize_deposit_asset_symbols(
    asset_symbol: Option<String>,
    asset_symbols: Option<Vec<String>>,
) -> AppResult<Vec<String>> {
    let mut symbols = Vec::new();
    let mut seen = HashSet::new();

    if let Some(values) = asset_symbols {
        for value in values {
            let Some(raw_symbol) = optional_string(Some(value)) else {
                continue;
            };
            let symbol = normalize_asset_symbol(&raw_symbol)?;
            if seen.insert(symbol.clone()) {
                symbols.push(symbol);
            }
        }
    }

    if symbols.is_empty()
        && let Some(raw_symbol) = optional_string(asset_symbol)
    {
        let symbol = normalize_asset_symbol(&raw_symbol)?;
        if seen.insert(symbol.clone()) {
            symbols.push(symbol);
        }
    }

    if symbols.len() > 50 {
        return Err(AppError::Validation(
            "asset_symbols cannot contain more than 50 assets".to_owned(),
        ));
    }

    Ok(symbols)
}

/// 将充值网络、地址组、资产集合、状态、排序和时间映射为配置审计快照。
/// 快照不包含地址池明细；应用层在网络配置写事务中保存前后值。
/// 资产白名单以数组展开，因此收窄白名单的操作能在审计里逐项看出移除了哪些资产，
/// 但由于快照不含地址，无法从中判断这次收窄让多少已入池地址失去准入资格。
pub(crate) fn deposit_network_config_audit_json(
    config: &AdminDepositNetworkConfigResponse,
) -> Value {
    json!({
        "id": config.id,
        "network": config.network,
        "display_name": config.display_name,
        "address_group_code": config.address_group_code,
        "address_group_name": config.address_group_name,
        "asset_symbols": config.asset_symbols.0.clone(),
        "status": config.status,
        "sort_order": config.sort_order,
        "created_at": config.created_at.timestamp_millis(),
        "updated_at": config.updated_at.timestamp_millis(),
    })
}

/// 在请求提供了阶梯提现费时借规范化流程做一次可行性校验，并丢弃规范化结果只保留成败。
/// 未提供时直接视为通过。调用方之后仍需自行执行一次真正的规范化来取得落库值，这里不承担转换职责。
fn validate_optional_withdraw_fee_tiers(value: Option<&[WithdrawFeeTier]>) -> AppResult<()> {
    if let Some(tiers) = value {
        normalize_asset_withdraw_fee_tiers(tiers.to_vec())?;
    }
    Ok(())
}

/// 对可缺省的资产金额字段做非负校验，缺省即通过，用于创建请求中三项金额均可不填的场景。
/// field 仅用于拼接错误文案；具体判定完全委托给非负校验，本函数不做精度截断或上限判断。
fn validate_optional_asset_amount(value: Option<&BigDecimal>, field: &str) -> AppResult<()> {
    if let Some(value) = value {
        validate_asset_amount(value, field)?;
    }
    Ok(())
}

/// 断言资产金额不为负，是最小充值额与充提手续费共用的底层判定。
/// 零被视为合法值，因此免手续费可以直接配 0；上界与小数位数不在此限制，由资产精度和数据库列类型约束。
fn validate_asset_amount(value: &BigDecimal, field: &str) -> AppResult<()> {
    crate::numeric::ensure_amount_storage(value, field)?;
    if value < &BigDecimal::from(0) {
        return Err(AppError::Validation(format!(
            "{field} must be non-negative"
        )));
    }
    Ok(())
}
