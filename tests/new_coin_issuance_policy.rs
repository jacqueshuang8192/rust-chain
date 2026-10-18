use bigdecimal::BigDecimal;
use chrono::{TimeZone, Utc};
use exchange_api::modules::new_coin::domain::issuance::*;
use serde_json::json;
use std::str::FromStr;

fn policy() -> IssuancePolicy {
    serde_json::from_value(json!({"version":2,"timezone":"Asia/Hong_Kong","purchase_initial_price":"0.2","listing_reference_price":"0.5",
        "subscription":{"automatic":true,"at":1900000000000i64},"distribution":{"automatic":false,"at":1900001000000i64},"purchase":{"automatic":true,"at":1900002000000i64},"listing":{"automatic":false,"at":1900003000000i64},"lock":{"kind":"months","count":3},"fee":{"kind":"profit","rate":"0.2"}})).unwrap()
}
fn d(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

#[test]
fn immutable_policy_validates_all_nodes_and_retains_manual_gates() {
    let mut p = policy();
    p.validate(8).unwrap();
    assert_eq!(p.next("distribution").unwrap().0, "purchase");
    assert!(!p.next("subscription").unwrap().1.automatic);
    assert!(p.next("listed").is_none());
    p.distribution.at = p.subscription.at;
    assert!(p.validate(8).is_err());
    p = policy();
    p.timezone = "server-local".into();
    assert!(p.validate(8).is_err());
    p = policy();
    p.fee = ExitFee::Profit { rate: d("1.01") };
    assert!(p.validate(8).is_err());
    p = policy();
    p.fee = ExitFee::Fixed { amount: d("0.001") };
    assert!(p.validate(2).is_err());
}
#[test]
fn duration_starts_at_original_acquisition_and_clamps_calendar_months() {
    let mut p = policy();
    p.lock = HoldingLock::Months { count: 1 };
    let acquired = Utc.with_ymd_and_hms(2026, 1, 31, 3, 0, 0).unwrap();
    assert_eq!(
        p.unlock_at(acquired).unwrap(),
        Some(Utc.with_ymd_and_hms(2026, 2, 28, 3, 0, 0).unwrap())
    );
    p.lock = HoldingLock::Weeks { count: 3 };
    assert_eq!(
        p.unlock_at(acquired).unwrap(),
        Some(acquired + chrono::Duration::weeks(3))
    );
    p.lock = HoldingLock::OnListing;
    assert_eq!(p.unlock_at(acquired).unwrap(), None);
    p.lock = HoldingLock::None;
    assert_eq!(p.unlock_at(acquired).unwrap(), Some(acquired));
    p.timezone = "America/New_York".into();
    p.lock = HoldingLock::Months { count: 1 };
    let source = Utc.with_ymd_and_hms(2026, 10, 1, 5, 30, 0).unwrap();
    assert_eq!(
        p.unlock_at(source).unwrap(),
        Some(Utc.with_ymd_and_hms(2026, 11, 1, 6, 30, 0).unwrap())
    );
}
#[test]
fn selected_batch_ratios_and_exit_fees_conserve_amounts() {
    for (ratio, qty, refund) in [
        ("0", "0", "1000"),
        ("50", "2500", "500"),
        ("80", "4000", "200"),
        ("100", "5000", "0"),
    ] {
        let allocated = allocation_quantity(&d("5000"), &d(ratio), 8).unwrap();
        assert_eq!(allocated, d(qty));
        assert_eq!(d("1000") - allocated * d("0.2"), d(refund));
    }
    assert_eq!(allocation_quantity(&d("3"), &d("50"), 0).unwrap(), d("1"));
    assert!(allocation_quantity(&d("3"), &d("101"), 8).is_err());
    assert_eq!(
        exit_amounts(&ExitFee::Profit { rate: d("0.2") }, &d("150"), &d("100"), 2).unwrap(),
        (d("10"), d("140"))
    );
    assert_eq!(
        exit_amounts(&ExitFee::Profit { rate: d("0.2") }, &d("80"), &d("100"), 2).unwrap(),
        (d("0"), d("80"))
    );
    assert!(exit_amounts(&ExitFee::Fixed { amount: d("81") }, &d("80"), &d("100"), 2).is_err());
}
