use super::*;

#[test]
fn exact_chain_amount_and_limits() {
    assert_eq!(
        raw_amount("9007199254740993000000000000000001", 10, 18)
            .unwrap()
            .to_string(),
        "9007199254740993.000000000000000001"
    );
    assert_eq!(
        raw_amount("1", 16, 18).unwrap(),
        "0.000000000000000001".parse::<BigDecimal>().unwrap()
    );
    assert!(raw_amount("-1", 10, 6).is_err());
    assert!(raw_amount("1", 10, 19).is_err());
    assert!(raw_amount(&"f".repeat(64), 16, 18).is_err());
}

#[test]
fn network_address_identity_and_checksums() {
    assert_eq!(
        canonical_address("eth", "0xABCDEFabcdefABCDEFabcdefABCDEFabcdefABCD").unwrap(),
        "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
    );
    let encoded = tron_address(&[1; 20]).unwrap();
    assert_eq!(canonical_address("tron", &encoded).unwrap(), encoded);
    assert!(canonical_address("tron", "invalid").is_err());
    assert_eq!(
        canonical_address("solana", "11111111111111111111111111111111").unwrap(),
        "11111111111111111111111111111111"
    );
    assert!(canonical_address("solana", "0abc").is_err());
    assert!(canonical_address("btc", "1BoatSLRHtKNngkdXEeobR76b53LETtpyT").is_ok());
    assert!(canonical_address("btc", "1BoatSLRHtKNngkdXEeobR76b53LETtpyX").is_err());
    assert_ne!(identity_key(&["Ab"]), identity_key(&["ab"]));
    assert_ne!(identity_key(&["a", "bc"]), identity_key(&["ab", "c"]));
}

#[test]
fn payment_window_is_half_open() {
    let start = Utc::now();
    let end = start + chrono::Duration::seconds(1800);
    assert!(within_window(start, start, end));
    assert!(!within_window(end, start, end));
    assert!(!within_window(
        start - chrono::Duration::milliseconds(1),
        start,
        end
    ));
}

#[test]
fn timed_deposit_permissions_and_string_only_amounts() {
    use crate::modules::admin::service::required_admin_permission;
    for (method, path, expected) in [
        (
            "PUT",
            "/wallet/deposit-settings/scanners/eth",
            "wallet.networks.write",
        ),
        (
            "PUT",
            "/wallet/deposit-settings/assets",
            "wallet.networks.write",
        ),
        (
            "PUT",
            "/wallet/deposit-settings/addresses/1",
            "wallet.address_pool.write",
        ),
        (
            "POST",
            "/wallet/deposit-receipts/1/review",
            "wallet.deposits.review",
        ),
        ("GET", "/wallet/deposit-orders/1", "wallet.deposits.read"),
    ] {
        assert_eq!(
            required_admin_permission(method, path).as_deref(),
            Some(expected)
        );
    }
    type Input = crate::modules::wallet::presentation::timed_deposits::CreateDepositOrder;
    assert!(serde_json::from_value::<Input>(serde_json::json!({"network":"eth","asset_symbol":"ETH","amount":50,"idempotency_key":"x"})).is_err());
    assert!(serde_json::from_value::<Input>(serde_json::json!({"network":"eth","asset_symbol":"ETH","amount":"50","idempotency_key":"x"})).is_ok());
}
