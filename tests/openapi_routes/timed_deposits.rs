use super::*;

#[tokio::test]
async fn timed_deposit_openapi_preserves_decimal_text_and_protected_routes() {
    let doc = openapi_json().await;
    for (path, method) in [
        ("/api/v1/wallet/deposit-options", "get"),
        ("/api/v1/wallet/deposit-orders", "post"),
        ("/admin/api/v1/wallet/deposit-settings", "get"),
        ("/admin/api/v1/wallet/deposit-receipts/{id}/review", "post"),
    ] {
        assert!(operation_has_bearer_security(&doc, path, method));
    }
    let schemas = &doc["components"]["schemas"];
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
}
