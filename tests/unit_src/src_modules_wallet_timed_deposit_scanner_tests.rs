use super::*;

#[path = "src_modules_wallet_base_native_scanner_tests.rs"]
mod base_native_tests;
#[path = "src_modules_wallet_deposit_provider_tests.rs"]
mod provider_tests;
#[path = "src_modules_wallet_scanner_recovery_tests.rs"]
mod recovery_tests;

#[test]
fn bitcoin_value_never_uses_float() {
    let value: Value = serde_json::from_str("9007199254740993.00000001").unwrap();
    assert_eq!(
        bitcoin::parse_value(&value).unwrap().to_string(),
        "9007199254740993.00000001"
    );
    assert!(bitcoin::parse_value(&json!("1")).is_err());
    assert!(bitcoin::parse_value(&serde_json::from_str("0.000000001").unwrap()).is_err());
}

#[test]
fn transfer_topic_validates_padding_and_confirmation_bounds() {
    assert_eq!(
        evm::topic_address("eth", &format!("0x{}{}", "0".repeat(24), "a".repeat(40))).unwrap(),
        format!("0x{}", "a".repeat(40))
    );
    assert!(evm::topic_address("eth", &"a".repeat(64)).is_err());
    assert!(confirmations(1, 2).is_err());
    assert_eq!(confirmations(10, 8).unwrap(), 3);
}

fn node(server: &wiremock::MockServer, network: &str) -> Node {
    Node {
        gate: None,
        network: network.into(),
        endpoint: server.uri(),
        api_key: None,
        client: reqwest::Client::new(),
    }
}
async fn rpc(server: &wiremock::MockServer, method: &str, result: Value) {
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::body_partial_json(
            json!({"method":method}),
        ))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_json(json!({"jsonrpc":"2.0","id":1,"result":result})),
        )
        .mount(server)
        .await;
}
fn asset(kind: &str, token: &str, decimals: i32) -> ChainAsset {
    ChainAsset {
        id: 1,
        asset_symbol: "TEST".into(),
        asset_kind: kind.into(),
        token_address: token.into(),
        decimals,
    }
}

#[tokio::test]
async fn evm_receipts_include_native_internal_transfers_and_exclude_failed_ancestors() {
    let server = wiremock::MockServer::start().await;
    let address = "0x2222222222222222222222222222222222222222";
    let contract = "0x1111111111111111111111111111111111111111";
    rpc(&server,"eth_getBlockByNumber",json!({"number":"0xa","hash":"block10","parentHash":"block9","timestamp":"0x6553f100","transactions":["tx1"]})).await;
    rpc(&server,"eth_getLogs",json!([{"blockHash":"block10","blockNumber":"0xa","address":contract,"transactionHash":"tx1","logIndex":"0x0","removed":false,"topics":[format!("0x{TRANSFER_TOPIC}"),format!("0x{}","0".repeat(64)),format!("0x{}{}","0".repeat(24),&address[2..])],"data":format!("0x{:064x}",100_000_000u64)}])).await;
    rpc(&server,"debug_traceBlockByHash",json!([{"txHash":"tx1","result":{"type":"CALL","from":contract,"to":address,"value":"0xde0b6b3a7640000","calls":[{"type":"CALL","from":contract,"to":address,"value":"0x1bc16d674ec80000"},{"type":"CALL","to":address,"value":"0xde0b6b3a7640000","error":"execution reverted","calls":[{"type":"CALL","to":address,"value":"0xde0b6b3a7640000"}]}]}}])).await;
    let context = Context {
        network: "eth".into(),
        tip: 22,
        assets: vec![asset("token", contract, 6), asset("native", "", 18)],
        addresses: [address.to_owned()].into(),
    };
    let block = evm::block(&node(&server, "eth"), 10, &context)
        .await
        .unwrap();
    assert_eq!(block.receipts.len(), 3);
    assert_eq!(block.receipts[0].amount, bigdecimal::BigDecimal::from(100));
    assert_eq!(block.receipts[1].amount, bigdecimal::BigDecimal::from(1));
    assert_eq!(block.receipts[2].amount, bigdecimal::BigDecimal::from(2));
    assert_eq!(
        block
            .receipts
            .iter()
            .map(|r| r.event_index)
            .collect::<HashSet<_>>()
            .len(),
        3
    );
    assert_eq!(block.receipts[0].confirmations, 13);
    rpc(&server, "eth_chainId", json!("0x1")).await;
    assert!(evm::head(&node(&server, "base"), &[]).await.is_err());
}

#[tokio::test]
async fn bitcoin_outputs_are_independent_and_coinbase_is_not_a_deposit() {
    let server = wiremock::MockServer::start().await;
    let address = "1BoatSLRHtKNngkdXEeobR76b53LETtpyT";
    rpc(&server, "getblockhash", json!("block10")).await;
    rpc(&server,"getblock",json!({"hash":"block10","height":10,"confirmations":6,"previousblockhash":"block9","time":1700000000,"tx":[{"txid":"mining","vin":[{"coinbase":"script"}],"vout":[{"n":0,"value":50,"scriptPubKey":{"address":address}}]},{"txid":"payment","vin":[{"txid":"source","vout":1}],"vout":[{"n":0,"value":serde_json::from_str::<Value>("0.00000001").unwrap(),"scriptPubKey":{"address":address}},{"n":1,"value":2,"scriptPubKey":{"address":address}}]}]})).await;
    let context = Context {
        network: "btc".into(),
        tip: 15,
        assets: vec![asset("native", "", 8)],
        addresses: [address.into()].into(),
    };
    let block = bitcoin::block(&node(&server, "btc"), 10, &context)
        .await
        .unwrap();
    assert_eq!(block.receipts.len(), 2);
    assert_eq!(block.receipts[0].event_index, 0);
    assert_eq!(block.receipts[1].event_index, 1);
    assert_eq!(
        block.receipts[0].amount,
        "0.00000001".parse::<bigdecimal::BigDecimal>().unwrap()
    );
    assert!(block.receipts.iter().all(|r| r.tx_hash == "payment"));
}

#[tokio::test]
async fn tron_reads_executed_transfer_logs_and_requires_complete_receipts() {
    use crate::modules::wallet::domain::timed_deposits::tron_address;
    let server = wiremock::MockServer::start().await;
    let address = tron_address(&[2; 20]).unwrap();
    let contract = tron_address(&[1; 20]).unwrap();
    wiremock::Mock::given(wiremock::matchers::path("/wallet/getblockbynum")).respond_with(wiremock::ResponseTemplate::new(200).set_body_json(json!({"blockID":"block10","block_header":{"raw_data":{"number":10,"parentHash":"block9","timestamp":1700000000000u64}},"transactions":[{"txID":"payment"}]}))).mount(&server).await;
    wiremock::Mock::given(wiremock::matchers::path("/wallet/gettransactioninfobyblocknum")).respond_with(wiremock::ResponseTemplate::new(200).set_body_json(json!([{"id":"payment","blockNumber":10,"blockTimeStamp":1700000000000u64,"receipt":{"result":"SUCCESS"},"log":[{"address":hex::encode([1;20]),"topics":[TRANSFER_TOPIC,"0".repeat(64),format!("{}{}","0".repeat(24),hex::encode([2;20]))],"data":format!("{:064x}",100_000_000u64)}]}]))).mount(&server).await;
    let context = Context {
        network: "tron".into(),
        tip: 29,
        assets: vec![asset("token", &contract, 6)],
        addresses: [address.clone()].into(),
    };
    let block = tron::block(&node(&server, "tron"), 10, &context)
        .await
        .unwrap();
    assert_eq!(block.receipts.len(), 1);
    assert_eq!(block.receipts[0].address, address);
    assert_eq!(block.receipts[0].amount, bigdecimal::BigDecimal::from(100));
    server.reset().await;
    wiremock::Mock::given(wiremock::matchers::path("/wallet/getblockbynum")).respond_with(wiremock::ResponseTemplate::new(200).set_body_json(json!({"blockID":"block10","block_header":{"raw_data":{"number":10,"parentHash":"block9","timestamp":1700000000000u64}},"transactions":[{"txID":"payment"}]}))).mount(&server).await;
    wiremock::Mock::given(wiremock::matchers::path(
        "/wallet/gettransactioninfobyblocknum",
    ))
    .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(json!([])))
    .mount(&server)
    .await;
    assert!(
        tron::block(&node(&server, "tron"), 10, &context)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn solana_validates_program_and_keeps_inner_transfers_separate() {
    let server = wiremock::MockServer::start().await;
    let address = "11111111111111111111111111111111";
    let mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    let transfer = json!({"programId":"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA","parsed":{"type":"transferChecked","info":{"source":"source","destination":"ata","mint":mint,"tokenAmount":{"amount":"100000000","decimals":6}}}});
    let native = json!({"programId":"11111111111111111111111111111111","parsed":{"type":"transfer","info":{"source":"sender","destination":address,"lamports":1000000000}}});
    let mut forged = transfer.clone();
    forged["programId"] = json!("not-the-token-program");
    let good = json!({"transaction":{"signatures":["signature"],"message":{"accountKeys":[{"pubkey":"ata"}],"instructions":[native,forged]}},"meta":{"err":null,"preTokenBalances":[],"postTokenBalances":[{"accountIndex":0,"mint":mint,"owner":address,"uiTokenAmount":{"decimals":6}}],"innerInstructions":[{"index":0,"instructions":[transfer]}]}});
    let mut failed = good.clone();
    failed["meta"]["err"] = json!({"InstructionError":[0,"failed"]});
    rpc(&server,"getBlock",json!({"blockhash":"block10","previousBlockhash":"block9","blockTime":1700000000,"transactions":[good,failed]})).await;
    let context = Context {
        network: "solana".into(),
        tip: 41,
        assets: vec![asset("native", "", 9), asset("token", mint, 6)],
        addresses: [address.into()].into(),
    };
    let block = solana::block(&node(&server, "solana"), 10, &context)
        .await
        .unwrap();
    assert_eq!(block.receipts.len(), 2);
    assert_eq!(block.receipts[0].amount, bigdecimal::BigDecimal::from(1));
    assert_eq!(block.receipts[1].amount, bigdecimal::BigDecimal::from(100));
    assert_ne!(block.receipts[0].event_index, block.receipts[1].event_index);
}

#[tokio::test]
async fn solana_rejects_invalid_slot_lists_and_skips_unconfigured_temporary_accounts() {
    let server = wiremock::MockServer::start().await;
    for slots in [
        json!([9, 10]),
        json!([11, 11]),
        json!([12, 11]),
        json!([11, 14]),
    ] {
        rpc(&server, "getBlocks", slots).await;
        assert!(node(&server, "solana").heights(10, 13).await.is_err());
        server.reset().await;
    }
    rpc(&server, "getBlocks", json!([10, 12])).await;
    assert_eq!(
        node(&server, "solana").heights(10, 13).await.unwrap(),
        vec![10, 12]
    );
    let token = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    rpc(&server,"getBlock",json!({"blockhash":"block10","previousBlockhash":"block9","blockTime":1700000000,"transactions":[{"transaction":{"signatures":["temporary-token"],"message":{"accountKeys":[],"instructions":[
        {"programId":token,"parsed":{"type":"initializeAccount3","info":{"account":"temporary","mint":"unconfigured","owner":"other-owner"}}},
        {"programId":token,"parsed":{"type":"transfer","info":{"source":"source","destination":"temporary","amount":"500"}}},
        {"programId":token,"parsed":{"type":"closeAccount","info":{"account":"temporary"}}}
    ]}},"meta":{"err":null,"preTokenBalances":[],"postTokenBalances":[],"innerInstructions":[]}}]})).await;
    let context = Context {
        network: "solana".into(),
        tip: 42,
        assets: vec![asset("native", "", 9)],
        addresses: ["receiver".into()].into(),
    };
    assert!(
        solana::block(&node(&server, "solana"), 10, &context)
            .await
            .unwrap()
            .receipts
            .is_empty()
    );
}

#[tokio::test]
async fn bitcoin_node_url_supports_basic_auth_without_exposing_credentials() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::header("authorization", "Basic bG9jYWw6Zml4dHVyZQ=="))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_json(json!({"jsonrpc":"2.0","id":1,"result":{"chain":"main","initialblockdownload":false,"headers":100,"blocks":100}})))
        .mount(&server).await;
    let mut authenticated = node(&server, "btc");
    authenticated.endpoint = server.uri().replacen("http://", "http://local:fixture@", 1);
    assert_eq!(bitcoin::head(&authenticated).await.unwrap(), 100);
}

#[tokio::test]
async fn tron_http_failures_preserve_status_operation_and_never_expose_credentials() {
    use wiremock::{Mock, ResponseTemplate, matchers};
    let server = wiremock::MockServer::start().await;
    for (status, hint) in [
        (401, "鉴权失败"),
        (403, "访问被拒绝"),
        (404, "接口不存在"),
        (405, "请求方式不支持"),
        (429, "请求限流"),
        (502, "上游网关异常"),
        (302, "重定向"),
        (418, "节点拒绝请求"),
    ] {
        server.reset().await;
        Mock::given(matchers::method("POST"))
            .and(matchers::path("/private-endpoint/wallet/getnowblock"))
            .and(matchers::header("TRON-PRO-API-KEY", "private-api-key"))
            .and(matchers::header("accept", "application/json"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("Location", "https://invalid.example/private-location")
                    .set_body_string("<html>private-api-key private-response-secret</html>"),
            )
            .expect(1)
            .mount(&server)
            .await;
        let mut configured = node(&server, "tron");
        configured.endpoint = format!("{}/private-endpoint", server.uri());
        configured.api_key = Some("private-api-key".into());
        configured.client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        let error = configured
            .post("/wallet/getnowblock", json!({}))
            .await
            .unwrap_err();
        let AppError::Internal(message) = error else {
            panic!("expected safe scanner diagnostic");
        };
        assert!(message.contains(&format!("tron /wallet/getnowblock HTTP {status}")));
        assert!(message.contains(hint), "{message}");
        assert!(message.contains("API Key 已配置"));
        assert!(message.chars().count() <= 255);
        for secret in ["private-", "<html>", &server.uri()] {
            assert!(!message.contains(secret), "{message}");
        }
        server.verify().await;
    }
}

#[tokio::test]
async fn tron_request_keeps_api_key_header_and_successful_json() {
    use wiremock::{Mock, ResponseTemplate, matchers};
    let server = wiremock::MockServer::start().await;
    Mock::given(matchers::path("/wallet/getnowblock"))
        .and(matchers::header("TRON-PRO-API-KEY", "saved-test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"blockID":"confirmed"})))
        .expect(1)
        .mount(&server)
        .await;
    let mut configured = node(&server, "tron");
    configured.api_key = Some("saved-test-key".into());
    assert_eq!(
        configured
            .post("/wallet/getnowblock", json!({}))
            .await
            .unwrap(),
        json!({"blockID":"confirmed"})
    );
    let requests = server.received_requests().await.unwrap();
    assert!(!requests[0].headers.contains_key("authorization"));
}

#[tokio::test]
async fn node_rpc_http_failure_identifies_method_without_echoing_params() {
    let server = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::method("POST"))
        .respond_with(wiremock::ResponseTemplate::new(429))
        .mount(&server)
        .await;
    let error = node(&server, "eth")
        .rpc("eth_getLogs", json!(["private-request-param"]))
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("eth eth_getLogs HTTP 429"));
    assert!(error.contains("API Key 未配置"));
    assert!(!error.contains("private-request-param"));
}

#[tokio::test]
async fn trongrid_403_hint_is_host_scoped_and_does_not_assert_key_is_invalid() {
    let server = wiremock::MockServer::start().await;
    let mut configured = node(&server, "tron");
    configured.endpoint = "https://api.trongrid.io/private-path?api_key=private-key".into();
    configured.api_key = Some("private-key".into());
    let message = configured
        .http_failure(
            "/wallet/gettransactioninfobyblocknum",
            reqwest::StatusCode::FORBIDDEN,
        )
        .to_string();
    assert!(message.contains("TronGrid"));
    assert!(message.contains("IP/接口白名单"));
    assert!(message.contains("请求频率和额度"));
    assert!(!message.contains("private-"));
    assert!(message.chars().count() <= 255);
    configured.endpoint = "https://api.trongrid.io.invalid.example".into();
    assert!(
        !configured
            .http_failure("/wallet/getnowblock", reqwest::StatusCode::FORBIDDEN)
            .to_string()
            .contains("TronGrid")
    );
}
