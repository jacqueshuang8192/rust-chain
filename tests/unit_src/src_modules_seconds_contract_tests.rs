use super::{repository, service};
use bigdecimal::BigDecimal;
use chrono::{TimeDelta, TimeZone, Utc};
use std::str::FromStr;

#[test]
fn numeric_safety_seconds_stake_storage_ignores_only_trailing_zeros() {
    for value in [
        "1e-18",
        "99999999999999999999.999999999999999999",
        "1.000000000000000000000",
    ] {
        service::validate_stake_amount(&value.parse().unwrap()).unwrap();
    }
    for value in ["1e20", "1e-19", "1e100000", "1e-4294967297"] {
        assert!(service::validate_stake_amount(&value.parse().unwrap()).is_err());
    }
}

#[test]
fn gross_payout_budget_preserves_net_rate_and_capacity_precision() {
    let decimal = |value: &str| BigDecimal::from_str(value).unwrap();
    assert_eq!(
        service::maximum_gross_payout(&decimal("100"), &decimal("0.4")),
        decimal("140")
    );
    assert_eq!(
        service::maximum_gross_payout(&decimal("100"), &decimal("1.4")),
        decimal("240")
    );
    assert_eq!(
        service::maximum_gross_payout(&decimal("0.000000000000000001"), &decimal("0.4")),
        decimal("0.000000000000000002")
    );
    for value in [None, Some(decimal("0")), Some(decimal("1.2300"))] {
        service::validate_payout_capacity(value.as_ref(), 2).unwrap();
    }
    for value in ["-1", "1.001", "100000000000000000000"] {
        assert!(service::validate_payout_capacity(Some(&decimal(value)), 2).is_err());
    }
}

#[test]
fn manual_review_requires_evidence_based_settlement_mode() {
    assert!(service::ensure_manual_settlement_allowed("manual_review", true).is_ok());
    assert!(service::ensure_manual_settlement_allowed("manual_review", false).is_err());
    assert!(service::ensure_manual_settlement_allowed("opened", false).is_ok());
    assert!(service::ensure_manual_settlement_allowed("settled", true).is_ok());
    assert!(service::ensure_manual_settlement_allowed("refunded", true).is_err());
}

fn snapshot(expires_at: chrono::DateTime<Utc>) -> repository::SecondsContractSettlementPriceRow {
    repository::SecondsContractSettlementPriceRow {
        id: Some(1),
        symbol: "BTCUSDT".to_owned(),
        price: BigDecimal::from(100),
        source: "bitget".to_owned(),
        observed_at: expires_at,
        generation: Some(1),
        source_version: "event-v1".to_owned(),
    }
}

#[test]
fn settlement_snapshot_validates_all_provenance_and_window_boundaries() {
    let expires_at = Utc.with_ymd_and_hms(2026, 8, 24, 12, 0, 0).unwrap();
    let valid = snapshot(expires_at);
    assert!(service::validate_settlement_price_snapshot(&valid, "btc-usdt", expires_at).is_ok());

    let mut default_source = valid.clone();
    default_source.source = "default".to_owned();
    assert!(
        service::validate_settlement_price_snapshot(&default_source, "BTCUSDT", expires_at).is_ok(),
        "default-generator ticks are archival settlement evidence"
    );

    let mut invalid = valid.clone();
    invalid.symbol = "ETHUSDT".to_owned();
    assert!(service::validate_settlement_price_snapshot(&invalid, "BTCUSDT", expires_at).is_err());

    invalid = valid.clone();
    invalid.source = "unknown".to_owned();
    assert!(service::validate_settlement_price_snapshot(&invalid, "BTCUSDT", expires_at).is_err());

    invalid = valid.clone();
    invalid.generation = Some(0);
    assert!(service::validate_settlement_price_snapshot(&invalid, "BTCUSDT", expires_at).is_err());

    invalid = valid.clone();
    invalid.source_version.clear();
    assert!(service::validate_settlement_price_snapshot(&invalid, "BTCUSDT", expires_at).is_err());

    invalid = valid.clone();
    invalid.observed_at = expires_at - TimeDelta::microseconds(1);
    assert!(service::validate_settlement_price_snapshot(&invalid, "BTCUSDT", expires_at).is_err());

    invalid = valid;
    invalid.observed_at = expires_at + TimeDelta::seconds(5);
    assert!(service::validate_settlement_price_snapshot(&invalid, "BTCUSDT", expires_at).is_err());
}

#[test]
fn payout_rate_is_net_profit_while_historical_order_snapshots_keep_their_value() {
    let stake = BigDecimal::from(100);
    let configured_net_rate = BigDecimal::from_str("0.4").unwrap();
    let historical_snapshot_rate = BigDecimal::from_str("1.4").unwrap();

    assert_eq!(
        service::seconds_contract_payout_amount(&stake, &configured_net_rate, "win", 18),
        BigDecimal::from(140)
    );
    assert_eq!(
        service::seconds_contract_payout_amount(&stake, &historical_snapshot_rate, "win", 18),
        BigDecimal::from(240)
    );
    assert_eq!(
        service::seconds_contract_payout_amount(&stake, &configured_net_rate, "loss", 18),
        BigDecimal::from(0)
    );
}

#[test]
fn control_result_normalization_accepts_clear_aliases_and_rejects_unknown() {
    for value in ["win", " WIN ", "Win"] {
        assert_eq!(
            service::normalize_control_result(value).unwrap(),
            Some("win".to_owned())
        );
    }
    for value in ["loss", "Loss"] {
        assert_eq!(
            service::normalize_control_result(value).unwrap(),
            Some("loss".to_owned())
        );
    }
    for value in ["clear", "auto", "none", "", "  "] {
        assert_eq!(service::normalize_control_result(value).unwrap(), None);
    }
    for value in ["draw", "loss ", "winloss"] {
        if value == "loss " {
            continue;
        }
        assert!(service::normalize_control_result(value).is_err());
    }
}

/// 受控判定价的步长按交易对价格精度换算；测试统一用两位精度，步长为 0.01。
fn control_step() -> BigDecimal {
    service::price_step_for_precision(2).unwrap()
}

#[test]
fn price_step_for_precision_matches_pair_scale() {
    assert_eq!(
        service::price_step_for_precision(0).unwrap(),
        BigDecimal::from(1)
    );
    assert_eq!(control_step(), BigDecimal::from_str("0.01").unwrap());
    assert_eq!(
        service::price_step_for_precision(8).unwrap(),
        BigDecimal::from_str("0.00000001").unwrap()
    );
    assert!(service::price_step_for_precision(-1).is_err());
    assert!(service::price_step_for_precision(19).is_err());
}

#[test]
fn control_offset_units_are_stable_and_bounded() {
    let units = service::control_price_offset_units(7, "down", "loss");
    assert_eq!(
        units,
        service::control_price_offset_units(7, "down", "loss")
    );
    assert!((1..=9).contains(&units));
    let spread: Vec<u8> = (1..=200)
        .map(|order_id| service::control_price_offset_units(order_id, "up", "win"))
        .collect();
    assert!(spread.iter().all(|value| (1..=9).contains(value)));
    assert!(spread.iter().any(|value| *value != spread[0]));
}

#[test]
fn resolve_settlement_outcome_without_control_follows_market_price() {
    let entry = BigDecimal::from(100);
    let market = BigDecimal::from(101);
    let resolution =
        service::resolve_settlement_outcome(1, "up", &entry, &market, None, None, &control_step())
            .unwrap();
    assert_eq!(resolution.result, "win");
    assert_eq!(resolution.market_result, "win");
    assert_eq!(resolution.settlement_price, market);
    assert!(!resolution.controlled);
}

#[test]
fn resolve_settlement_outcome_prefers_order_control_over_user_control() {
    let entry = BigDecimal::from(100);
    let market = BigDecimal::from(101);
    let resolution = service::resolve_settlement_outcome(
        1,
        "up",
        &entry,
        &market,
        Some("loss"),
        Some("win"),
        &control_step(),
    )
    .unwrap();
    assert_eq!(resolution.result, "loss");
    assert_eq!(resolution.market_result, "win");
    assert!(resolution.controlled);
    let units = i64::from(service::control_price_offset_units(1, "up", "loss"));
    assert_eq!(
        resolution.settlement_price,
        entry.clone() - control_step() * BigDecimal::from(units)
    );
}

#[test]
fn resolve_settlement_outcome_keeps_market_price_when_control_already_matches() {
    let entry = BigDecimal::from(100);
    let market = BigDecimal::from_str("99.5").unwrap();
    let resolution = service::resolve_settlement_outcome(
        1,
        "up",
        &entry,
        &market,
        Some("loss"),
        None,
        &control_step(),
    )
    .unwrap();
    assert_eq!(resolution.result, "loss");
    assert_eq!(resolution.settlement_price, market);
    assert!(resolution.controlled);
}

#[test]
fn controlled_loss_offsets_entry_tail_within_one_to_nine_units() {
    // 开仓价 65532.44、两位精度、用户看跌、控制输：结算价应为开仓价加上 1 到 9 个尾数单位。
    let entry = BigDecimal::from_str("65532.44").unwrap();
    let market = BigDecimal::from_str("65530.00").unwrap();
    let resolution = service::resolve_settlement_outcome(
        42,
        "down",
        &entry,
        &market,
        Some("loss"),
        None,
        &control_step(),
    )
    .unwrap();
    assert_eq!(resolution.result, "loss");
    assert_eq!(resolution.market_result, "win");
    let units = i64::from(service::control_price_offset_units(42, "down", "loss"));
    assert_eq!(
        resolution.settlement_price,
        entry.clone() + control_step() * BigDecimal::from(units)
    );
    assert!(resolution.settlement_price > entry);
    assert!(resolution.settlement_price < BigDecimal::from_str("65533").unwrap());
}

#[test]
fn controlled_win_offsets_entry_on_the_profitable_side() {
    let entry = BigDecimal::from(100);
    let up_win = service::resolve_settlement_outcome(
        9,
        "up",
        &entry,
        &BigDecimal::from_str("99").unwrap(),
        Some("win"),
        None,
        &control_step(),
    )
    .unwrap();
    let up_units = i64::from(service::control_price_offset_units(9, "up", "win"));
    assert_eq!(up_win.result, "win");
    assert_eq!(
        up_win.settlement_price,
        entry.clone() + control_step() * BigDecimal::from(up_units)
    );

    let down_win = service::resolve_settlement_outcome(
        9,
        "down",
        &entry,
        &BigDecimal::from_str("101").unwrap(),
        Some("win"),
        None,
        &control_step(),
    )
    .unwrap();
    let down_units = i64::from(service::control_price_offset_units(9, "down", "win"));
    assert_eq!(down_win.result, "win");
    assert_eq!(
        down_win.settlement_price,
        entry.clone() - control_step() * BigDecimal::from(down_units)
    );
}

#[test]
fn resolve_settlement_outcome_keeps_controlled_down_win_price_positive() {
    let entry = BigDecimal::from_str("0.001").unwrap();
    let resolution = service::resolve_settlement_outcome(
        3,
        "down",
        &entry,
        &BigDecimal::from_str("0.002").unwrap(),
        Some("win"),
        None,
        &control_step(),
    )
    .unwrap();
    assert_eq!(resolution.result, "win");
    assert!(resolution.settlement_price > BigDecimal::from(0));
    assert!(resolution.settlement_price < entry);
}
