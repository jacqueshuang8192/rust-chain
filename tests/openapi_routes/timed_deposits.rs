use super::*;

#[tokio::test]
async fn timed_deposit_openapi_preserves_decimal_text_and_protected_routes() {
    let doc = openapi_json().await;
    for (path, method) in [
        ("/api/v1/wallet/deposit-options", "get"),
        ("/api/v1/wallet/deposit-orders", "post"),
        ("/api/v1/wallet/deposit-orders/{id}/cancel", "post"),
        ("/admin/api/v1/wallet/deposit-settings", "get"),
        (
            "/admin/api/v1/wallet/deposit-settings/scanners/{network}/recovery",
            "post",
        ),
        ("/admin/api/v1/wallet/deposit-receipts/{id}/review", "post"),
    ] {
        assert!(operation_has_bearer_security(&doc, path, method));
    }
    for name in [
        "recovery_mode",
        "backfill_cursor_height",
        "backfill_target_height",
        "backfill_last_error",
        "backfill_blocked",
    ] {
        assert!(
            doc["components"]["schemas"]["ScannerView"]["properties"]
                .get(name)
                .is_some()
        );
    }
    let schemas = &doc["components"]["schemas"];
    assert_eq!(
        schemas["ScannerView"]["properties"]["streams"]["type"],
        "array"
    );
    assert!(
        schemas["ScannerStreamView"]["properties"]
            .get("cursor_height")
            .is_some()
    );
    assert!(
        schemas["RecoverScanner"]["properties"]
            .get("stream")
            .is_some()
    );
    assert!(
        !schemas["RecoverScanner"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "stream")
    );
    assert_eq!(
        schemas["ScannerView"]["properties"]["providers"]["type"],
        "array"
    );
    assert!(
        schemas["SaveScanner"]["properties"]
            .get("providers")
            .is_some()
    );
    for name in ["ScannerProviderView", "ScannerKeyView"] {
        assert!(schemas[name]["properties"].get("cooldown_until").is_some());
        for secret in [
            "endpoint",
            "endpoint_encrypted",
            "api_key",
            "api_key_encrypted",
        ] {
            assert!(schemas[name]["properties"].get(secret).is_none());
        }
    }
    assert_eq!(
        schemas["ScannerView"]["properties"]["scan_fresh"]["type"],
        "boolean"
    );
    assert_eq!(
        schemas["ScannerView"]["properties"]["checked_at"]["type"],
        "integer"
    );
    assert!(
        schemas["ScannerView"]["properties"]
            .get("cursor_height")
            .is_some()
    );
    assert_eq!(
        schemas["CreateDepositOrder"]["properties"]["amount"]["type"],
        "string"
    );
    assert_eq!(
        schemas["DepositOrder"]["properties"]["amount"]["type"],
        "string"
    );
    assert_eq!(
        schemas["DepositOrder"]["properties"]["expires_at"]["type"],
        "integer"
    );
    assert!(
        doc["paths"]["/admin/api/v1/wallet/deposit-orders"]["get"]["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "order_no")
    );
    assert!(
        schemas["ScannerView"]["properties"]
            .get("endpoint_encrypted")
            .is_none()
    );
    assert!(
        schemas["ScannerView"]["properties"]
            .get("api_key")
            .is_none()
    );
    assert_eq!(
        schemas["ScannerRecoveryQueued"]["properties"]["queued"]["type"],
        "boolean"
    );
}
