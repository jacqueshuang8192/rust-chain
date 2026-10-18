//! 第二版发行合同：实时认购、连续上市、原始购买时点锁仓及从卖出款扣费；纯规则不访问存储。
use crate::error::{AppError, AppResult};
use bigdecimal::{BigDecimal, RoundingMode};
use chrono::{DateTime, Duration, LocalResult, Months, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

/// 节点时间是 UTC 瞬间；automatic 仅推进阶段，派发始终需要代理选择订单确认。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseNode {
    pub automatic: bool,
    #[serde(with = "crate::time::unix_millis")]
    pub at: DateTime<Utc>,
}

/// 不可变锁仓口径；阶段条件依赖真实事件，周期从原始购买时刻起算。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HoldingLock {
    None,
    OnPurchase,
    OnListing,
    Weeks {
        count: u32,
    },
    Months {
        count: u32,
    },
    Fixed {
        #[serde(with = "crate::time::unix_millis")]
        at: DateTime<Utc>,
    },
}

/// 退出费用以计价资产扣除；比例用 0..1，固定费用不再要求预先付款。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExitFee {
    None,
    Profit {
        #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
        rate: BigDecimal,
    },
    Gross {
        #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
        rate: BigDecimal,
    },
    Fixed {
        #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
        amount: BigDecimal,
    },
}

/// 审核时原样发布的发行快照；认购初价只用于首次启动行情，上市参考价不得重置行情。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IssuancePolicy {
    pub version: u8,
    pub timezone: String,
    #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
    pub purchase_initial_price: BigDecimal,
    #[serde(deserialize_with = "crate::numeric::deserialize_decimal_text")]
    pub listing_reference_price: BigDecimal,
    pub subscription: PhaseNode,
    pub distribution: PhaseNode,
    pub purchase: PhaseNode,
    pub listing: PhaseNode,
    pub lock: HoldingLock,
    pub fee: ExitFee,
}

impl IssuancePolicy {
    /// 在提交和批准边界验证完整合同；金额、时间、时区和节点顺序非法时拒绝发布。
    pub fn validate(&self, quote_precision: i32) -> AppResult<()> {
        if self.version != 2 || self.timezone.parse::<Tz>().is_err() {
            return Err(AppError::Validation("发行版本或时区无效".into()));
        }
        for price in [&self.purchase_initial_price, &self.listing_reference_price] {
            crate::numeric::ensure_amount_storage(price, "发行价格")?;
            if price <= &BigDecimal::from(0)
                || price.normalized().fractional_digit_count() > i64::from(quote_precision)
            {
                return Err(AppError::Validation(
                    "发行价格必须为正且符合计价资产精度".into(),
                ));
            }
        }
        let nodes = [
            &self.subscription,
            &self.distribution,
            &self.purchase,
            &self.listing,
        ];
        for node in nodes {
            crate::time::ensure_timestamp_storage(&node.at, "阶段时间")?;
        }
        if nodes.windows(2).any(|w| w[0].at >= w[1].at) {
            return Err(AppError::Validation(
                "申购、派发、认购和上市时间必须依次递增".into(),
            ));
        }
        match &self.lock {
            HoldingLock::Weeks { count } if !(1..=520).contains(count) => {
                return Err(AppError::Validation("锁仓周数需为1至520".into()));
            }
            HoldingLock::Months { count } if !(1..=120).contains(count) => {
                return Err(AppError::Validation("锁仓月数需为1至120".into()));
            }
            HoldingLock::Fixed { at } => crate::time::ensure_timestamp_storage(at, "解锁时间")?,
            _ => {}
        }
        match &self.fee {
            ExitFee::Gross { rate } | ExitFee::Profit { rate } => {
                crate::numeric::ensure_amount_storage(rate, "退出费率")?;
                if rate < &BigDecimal::from(0) || rate > &BigDecimal::from(1) {
                    return Err(AppError::Validation("退出费率需介于0与1之间".into()));
                }
            }
            ExitFee::Fixed { amount } => {
                crate::numeric::ensure_amount_storage(amount, "退出费用")?;
                if amount < &BigDecimal::from(0)
                    || amount.normalized().fractional_digit_count() > i64::from(quote_precision)
                {
                    return Err(AppError::Validation("固定退出费用或精度无效".into()));
                }
            }
            ExitFee::None => {}
        }
        Ok(())
    }

    /// 返回唯一下一阶段；不会越过手动节点，也不会沿用旧版 distribution→listed 路径。
    pub fn next(&self, current: &str) -> Option<(&'static str, &PhaseNode)> {
        match current {
            "preheat" => Some(("subscription", &self.subscription)),
            "subscription" => Some(("distribution", &self.distribution)),
            "distribution" => Some(("purchase", &self.purchase)),
            "purchase" => Some(("listed", &self.listing)),
            _ => None,
        }
    }

    /// 固化时间锁到期点；阶段锁保留空值直到真实事件。月末钳制，夏令时歧义取较晚瞬间，缺口顺延。
    pub fn unlock_at(&self, acquired_at: DateTime<Utc>) -> AppResult<Option<DateTime<Utc>>> {
        let at = match self.lock {
            HoldingLock::None => Some(acquired_at),
            HoldingLock::OnPurchase | HoldingLock::OnListing => None,
            HoldingLock::Fixed { at } => Some(at),
            HoldingLock::Weeks { count } => {
                acquired_at.checked_add_signed(Duration::weeks(i64::from(count)))
            }
            HoldingLock::Months { count } => {
                let tz = self
                    .timezone
                    .parse::<Tz>()
                    .map_err(|_| AppError::Validation("锁仓时区无效".into()))?;
                let local = acquired_at
                    .with_timezone(&tz)
                    .naive_local()
                    .checked_add_months(Months::new(count))
                    .ok_or_else(|| AppError::Validation("锁仓月份超出范围".into()))?;
                let mut resolved = None;
                for minutes in 0..=180 {
                    let candidate = local
                        .checked_add_signed(Duration::minutes(minutes))
                        .ok_or_else(|| AppError::Validation("锁仓时间超出范围".into()))?;
                    resolved = match tz.from_local_datetime(&candidate) {
                        LocalResult::Single(at) => Some(at.with_timezone(&Utc)),
                        LocalResult::Ambiguous(_, later) => Some(later.with_timezone(&Utc)),
                        LocalResult::None => None,
                    };
                    if resolved.is_some() {
                        break;
                    }
                }
                resolved
            }
        };
        if let Some(at) = at {
            crate::time::ensure_timestamp_storage(&at, "锁仓到期时间")?;
        }
        if at.is_none() && !matches!(self.lock, HoldingLock::OnPurchase | HoldingLock::OnListing) {
            return Err(AppError::Validation("无法计算锁仓到期时间".into()));
        }
        Ok(at)
    }
}

/// 按选定百分比计算一次最终派发；向下截断到代币精度，剩余款由原订单价格精确退款。
pub fn allocation_quantity(
    requested: &BigDecimal,
    percent: &BigDecimal,
    precision: i32,
) -> AppResult<BigDecimal> {
    crate::numeric::ensure_amount_storage(percent, "派发比例")?;
    if percent < &BigDecimal::from(0)
        || percent > &BigDecimal::from(100)
        || !(0..=18).contains(&precision)
    {
        return Err(AppError::Validation("派发比例需介于0%与100%之间".into()));
    }
    Ok((requested * percent / BigDecimal::from(100))
        .with_scale_round(i64::from(precision), RoundingMode::Down))
}

/// 按成交时毛收入及原始成本计算扣费，利润为负时不收费；费用不得超过毛收入。
pub fn exit_amounts(
    fee: &ExitFee,
    gross: &BigDecimal,
    cost: &BigDecimal,
    precision: i32,
) -> AppResult<(BigDecimal, BigDecimal)> {
    let raw = match fee {
        ExitFee::None => BigDecimal::from(0),
        ExitFee::Fixed { amount } => amount.clone(),
        ExitFee::Gross { rate } => gross * rate,
        ExitFee::Profit { rate } => (gross - cost).max(BigDecimal::from(0)) * rate,
    };
    let charged = raw.with_scale_round(i64::from(precision), RoundingMode::Down);
    if gross <= &BigDecimal::from(0) || charged < 0 || &charged > gross {
        return Err(AppError::Validation("当前卖出款不足以支付退出费用".into()));
    }
    Ok((charged.clone(), gross - charged))
}
