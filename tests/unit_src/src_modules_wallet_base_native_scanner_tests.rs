use super::*;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

const RECEIVER: &str = "0x2222222222222222222222222222222222222222";
const SENDER: &str = "0x1111111111111111111111111111111111111111";
const OTHER: &str = "0x3333333333333333333333333333333333333333";
const ONE_ETH: &str = "0xde0b6b3a7640000";

fn transaction(hash: &str, index: u64, to: Option<&str>, value: &str) -> Value {
    json!({"hash":hash,"blockHash":"block10","blockNumber":"0xa",
        "transactionIndex":format!("0x{index:x}"),"from":SENDER,"to":to,"value":value})
}

fn transaction_receipt(transaction: &Value, status: &str) -> Value {
    json!({"transactionHash":transaction["hash"],"blockHash":transaction["blockHash"],
        "blockNumber":transaction["blockNumber"],"transactionIndex":transaction["transactionIndex"],
        "from":transaction["from"],"to":transaction["to"],"status":status})
}

fn context() -> Context {
    Context {
        network: "base".into(),
        tip: 22,
        assets: vec![asset("native", "", 18)],
        addresses: [RECEIVER.into()].into(),
    }
}

async fn mock_block(server: &MockServer, transactions: Value, hydrate: bool) {
    let mut hashes = transactions.clone();
    if hydrate && let Some(transactions) = transactions.as_array() {
        hashes = json!(
            transactions
                .iter()
                .map(|tx| { tx.get("hash").cloned().unwrap_or_else(|| tx.clone()) })
                .collect::<Vec<_>>()
        );
        let replies: Vec<_> = transactions
            .iter()
            .enumerate()
            .rev()
            .map(|(index, tx)| json!({"jsonrpc":"2.0","id":index + 1,"result":tx}))
            .collect();
        Mock::given(|request: &wiremock::Request| {
            serde_json::from_slice::<Value>(&request.body).is_ok_and(|body| {
                body.as_array().is_some_and(|calls| {
                    calls
                        .iter()
                        .all(|call| call["method"] == "eth_getTransactionByHash")
                })
            })
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(replies))
        .mount(server)
        .await;
    }
    Mock::given(matchers::method("POST"))
        .and(matchers::body_partial_json(
            json!({"method":"eth_getBlockByNumber","params":["0xa",false]}),
        ))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"jsonrpc":"2.0","id":1,
            "result":{"number":"0xa","hash":"block10","parentHash":"block9",
                "timestamp":"0x6553f100","transactions":hashes}})),
        )
        .expect(1)
        .mount(server)
        .await;
}

async fn mock_receipt(server: &MockServer, hash: &str, evidence: Value) {
    Mock::given(matchers::method("POST"))
        .and(matchers::body_partial_json(
            json!({"method":"eth_getTransactionReceipt","params":[hash]}),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"jsonrpc":"2.0","id":1,"result":evidence})),
        )
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn base_credits_only_successful_direct_native_transfers_and_keeps_erc20_logs() {
    let server = MockServer::start().await;
    let direct = transaction("direct", 0, Some(RECEIVER), ONE_ETH);
    let failed = transaction("failed", 1, Some(RECEIVER), ONE_ETH);
    let zero = transaction("zero", 2, Some(RECEIVER), "0x0");
    let mut self_transfer = transaction("self", 3, Some(RECEIVER), ONE_ETH);
    self_transfer["from"] = json!(RECEIVER);
    let creation = transaction("creation", 4, None, ONE_ETH);
    let mut internal_only = transaction("internal", 5, Some(OTHER), ONE_ETH);
    // An untrusted extra call tree cannot turn an internal payment into a direct one.
    internal_only["calls"] = json!([{"to":RECEIVER,"value":ONE_ETH}]);
    let exact = transaction("exact", 6, Some(RECEIVER), "0xde0b6b3a7640001");
    mock_block(
        &server,
        json!([
            direct,
            failed,
            zero,
            self_transfer,
            creation,
            internal_only,
            exact
        ]),
        true,
    )
    .await;
    mock_receipt(&server, "direct", transaction_receipt(&direct, "0x1")).await;
    mock_receipt(&server, "failed", transaction_receipt(&failed, "0x0")).await;
    mock_receipt(&server, "exact", transaction_receipt(&exact, "0x1")).await;
    rpc(&server,"eth_getLogs",json!([{"blockHash":"block10","blockNumber":"0xa","address":SENDER,"transactionHash":"direct","logIndex":"0x0","removed":false,"topics":[format!("0x{TRANSFER_TOPIC}"),format!("0x{}","0".repeat(64)),format!("0x{}{}","0".repeat(24),&RECEIVER[2..])],"data":format!("0x{:064x}",100_000_000u64)}])).await;
    let mut configured = context();
    configured.assets.push(asset("token", SENDER, 6));
    let block = evm::block(&node(&server, "base"), 10, &configured)
        .await
        .unwrap();
    assert_eq!(block.receipts.len(), 3);
    assert_eq!(block.receipts[0].asset_kind, "token");
    assert_eq!(block.receipts[0].amount, bigdecimal::BigDecimal::from(100));
    assert_eq!(block.receipts[0].event_index, 0);
    let native = &block.receipts[1..];
    assert_eq!(native[0].amount, bigdecimal::BigDecimal::from(1));
    assert_eq!(native[1].amount.to_string(), "1.000000000000000001");
    assert_eq!(native[0].tx_hash, "direct");
    assert_eq!(native[1].tx_hash, "exact");
    for receipt in native {
        assert_eq!(receipt.network, "base");
        assert_eq!(receipt.asset_kind, "native");
        assert_eq!(receipt.token_address, "");
        assert_eq!(receipt.address, RECEIVER);
        assert_eq!(receipt.event_index, 0x8000_0000);
        assert_eq!(receipt.block_hash, "block10");
        assert_eq!(receipt.block_height, 10);
        assert_eq!(receipt.block_time.timestamp(), 1_700_000_000);
        assert_eq!(receipt.confirmations, 13);
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 6);
    for request in requests {
        let request: Value = serde_json::from_slice(&request.body).unwrap();
        if let Some(calls) = request.as_array() {
            assert_eq!(calls.len(), 7);
            assert!(
                calls
                    .iter()
                    .all(|call| call["method"] == "eth_getTransactionByHash")
            );
            continue;
        }
        assert!(matches!(
            request["method"].as_str(),
            Some("eth_getBlockByNumber" | "eth_getLogs" | "eth_getTransactionReceipt")
        ));
    }
    server.verify().await;
}

#[rstest::rstest]
#[case("transactionHash", json!("other"))]
#[case("blockHash", json!("fork10"))]
#[case("blockNumber", json!("0xb"))]
#[case("transactionIndex", json!("0x1"))]
#[case("from", json!(OTHER))]
#[case("to", json!(OTHER))]
#[case("to", Value::Null)]
#[case("status", Value::Null)]
#[case("status", json!("0x2"))]
#[case("status", json!(1))]
#[tokio::test]
async fn base_rejects_mismatched_or_invalid_receipts(#[case] field: &str, #[case] value: Value) {
    let server = MockServer::start().await;
    let tx = transaction("direct", 0, Some(RECEIVER), ONE_ETH);
    let mut evidence = transaction_receipt(&tx, "0x1");
    evidence[field] = value;
    mock_block(&server, json!([tx]), true).await;
    mock_receipt(&server, "direct", evidence).await;
    assert!(
        evm::block(&node(&server, "base"), 10, &context())
            .await
            .is_err()
    );
    server.verify().await;
}

#[rstest::rstest]
#[case("hash", Value::Null)]
#[case("blockHash", json!("fork10"))]
#[case("blockNumber", json!("0xb"))]
#[case("transactionIndex", json!("0x1"))]
#[case("from", json!("invalid"))]
#[case("to", json!("invalid"))]
#[case("value", Value::Null)]
#[case("value", json!(1))]
#[case("value", json!("0x"))]
#[case("value", json!("0x-1"))]
#[case("value", json!("0x+1"))]
#[case("value", json!(format!("0x1{}", "0".repeat(64))))]
#[tokio::test]
async fn base_rejects_invalid_full_transactions(#[case] field: &str, #[case] value: Value) {
    let server = MockServer::start().await;
    let mut tx = transaction("direct", 0, Some(RECEIVER), ONE_ETH);
    tx[field] = value;
    mock_block(&server, json!([tx]), true).await;
    assert!(
        evm::block(&node(&server, "base"), 10, &context())
            .await
            .is_err()
    );
    assert!(
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .all(|request| {
                let body: Value = serde_json::from_slice(&request.body).unwrap();
                body["method"] != "eth_getTransactionReceipt"
            })
    );
    server.verify().await;
}

#[tokio::test]
async fn base_requires_full_transaction_list_and_explicit_recipient() {
    let server = MockServer::start().await;
    let mut missing_to = transaction("missing-to", 0, Some(RECEIVER), ONE_ETH);
    missing_to.as_object_mut().unwrap().remove("to");
    for transactions in [Value::Null, json!(["hash-only"]), json!([missing_to])] {
        server.reset().await;
        mock_block(&server, transactions, true).await;
        assert!(
            evm::block(&node(&server, "base"), 10, &context())
                .await
                .is_err()
        );
        server.verify().await;
    }
}

#[tokio::test]
async fn base_receipt_failure_rejects_the_entire_block_even_after_a_valid_transfer() {
    let server = MockServer::start().await;
    let first = transaction("first", 0, Some(RECEIVER), ONE_ETH);
    let second = transaction("second", 1, Some(RECEIVER), ONE_ETH);
    for response in [
        json!({"jsonrpc":"2.0","id":1,"result":null}),
        json!({"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"unavailable"}}),
    ] {
        server.reset().await;
        mock_block(&server, json!([first, second]), true).await;
        mock_receipt(&server, "first", transaction_receipt(&first, "0x1")).await;
        Mock::given(matchers::body_partial_json(
            json!({"method":"eth_getTransactionReceipt","params":["second"]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .expect(1)
        .mount(&server)
        .await;
        assert!(
            evm::block(&node(&server, "base"), 10, &context())
                .await
                .is_err()
        );
        server.verify().await;
    }
}

#[tokio::test]
async fn base_rejects_duplicate_transaction_hashes() {
    let server = MockServer::start().await;
    mock_block(
        &server,
        json!([
            transaction("duplicate", 0, Some(OTHER), "0x0"),
            transaction("duplicate", 1, Some(OTHER), "0x0")
        ]),
        true,
    )
    .await;
    assert!(
        evm::block(&node(&server, "base"), 10, &context())
            .await
            .is_err()
    );
    server.verify().await;
}

#[tokio::test]
async fn base_token_only_and_anchor_scans_do_not_request_full_transactions_or_native_receipts() {
    let server = MockServer::start().await;
    for assets in [vec![asset("token", SENDER, 6)], vec![]] {
        server.reset().await;
        mock_block(&server, json!(["hash-only"]), false).await;
        let count = if assets.is_empty() { 1 } else { 2 };
        if !assets.is_empty() {
            rpc(&server, "eth_getLogs", json!([])).await;
        }
        let mut configured = context();
        configured.assets = assets;
        assert!(
            evm::block(&node(&server, "base"), 10, &configured)
                .await
                .unwrap()
                .receipts
                .is_empty()
        );
        assert_eq!(server.received_requests().await.unwrap().len(), count);
        server.verify().await;
    }
}

#[tokio::test]
async fn base_direct_receipt_reuses_legacy_trace_root_identity() {
    let server = MockServer::start().await;
    let tx = transaction("payment", 0, Some(RECEIVER), ONE_ETH);
    mock_block(&server, json!([tx]), true).await;
    mock_receipt(&server, "payment", transaction_receipt(&tx, "0x1")).await;
    let direct = evm::block(&node(&server, "base"), 10, &context())
        .await
        .unwrap()
        .receipts
        .remove(0);
    server.verify().await;
    server.reset().await;
    // Ethereum retains the same trace-root parser previously used by Base.
    mock_block(&server, json!(["payment"]), false).await;
    rpc(
        &server,
        "debug_traceBlockByHash",
        json!([{"txHash":"payment","result":{
            "type":"CALL","from":SENDER,"to":RECEIVER,"value":ONE_ETH,
            "calls":[{"type":"CALL","from":RECEIVER,"to":OTHER,"value":"0x1"}]
        }}]),
    )
    .await;
    let mut legacy_context = context();
    legacy_context.network = "eth".into();
    let legacy = evm::block(&node(&server, "eth"), 10, &legacy_context)
        .await
        .unwrap()
        .receipts
        .remove(0);
    let mut legacy = serde_json::to_value(legacy).unwrap();
    legacy["network"] = json!("base");
    assert_eq!(serde_json::to_value(direct).unwrap(), legacy);
    server.verify().await;
}

#[tokio::test]
async fn transaction_batches_are_bounded_and_restore_request_order() {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .respond_with(|request: &wiremock::Request| {
            let requests: Vec<Value> = serde_json::from_slice(&request.body).unwrap();
            assert!(!requests.is_empty() && requests.len() <= 25);
            let replies: Vec<_> = requests.iter().rev().map(|request| {
                assert_eq!(request["method"], "eth_getTransactionByHash");
                json!({"jsonrpc":"2.0","id":request["id"],"result":{"hash":request["params"][0]}})
            }).collect();
            ResponseTemplate::new(200).set_body_json(replies)
        })
        .expect(5)
        .mount(&server)
        .await;
    let client = node(&server, "base");
    let result = client
        .rpc_batch(
            "eth_getTransactionByHash",
            (0..101).map(|i| json!([format!("tx{i}")])).collect(),
        )
        .await
        .unwrap();
    assert_eq!(
        result,
        (0..101)
            .map(|i| json!({"hash":format!("tx{i}")}))
            .collect::<Vec<_>>()
    );
    assert!(
        client
            .rpc_batch("eth_getTransactionByHash", vec![])
            .await
            .unwrap()
            .is_empty()
    );
    server.verify().await;
}

#[rstest::rstest]
#[case(json!([]))]
#[case(json!({"jsonrpc":"2.0","id":1,"result":{}}))]
#[case(json!([{"jsonrpc":"2.0","id":1,"result":{} }]))]
#[case(json!([{"jsonrpc":"2.0","id":1,"result":{} },{"jsonrpc":"2.0","id":1,"result":{}}]))]
#[case(json!([{"jsonrpc":"2.0","id":0,"result":{} },{"jsonrpc":"2.0","id":2,"result":{}}]))]
#[case(json!([{"jsonrpc":"2.0","id":1,"result":{} },{"jsonrpc":"2.0","id":3,"result":{}}]))]
#[case(json!([{"jsonrpc":"2.0","id":"1","result":{} },{"jsonrpc":"2.0","id":2,"result":{}}]))]
#[case(json!([{"jsonrpc":"1.0","id":1,"result":{} },{"jsonrpc":"2.0","id":2,"result":{}}]))]
#[case(json!([{"jsonrpc":"2.0","id":1,"result":{} },{"jsonrpc":"2.0","id":2,"result":null}]))]
#[case(json!([{"jsonrpc":"2.0","id":1,"result":{} },{"jsonrpc":"2.0","id":2}]))]
#[case(json!([{"jsonrpc":"2.0","id":1,"result":{} },{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"private-upstream-detail"}}]))]
#[tokio::test]
async fn transaction_batches_reject_incomplete_or_ambiguous_results(#[case] response: Value) {
    let server = MockServer::start().await;
    Mock::given(matchers::method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .expect(1)
        .mount(&server)
        .await;
    let error = node(&server, "base")
        .rpc_batch(
            "eth_getTransactionByHash",
            vec![json!(["first"]), json!(["second"])],
        )
        .await
        .unwrap_err();
    assert!(!error.to_string().contains("private-upstream-detail"));
    server.verify().await;
}

#[tokio::test]
async fn base_binds_hydrated_transactions_to_requested_hashes_and_accepts_empty_blocks() {
    let server = MockServer::start().await;
    mock_block(&server, json!(["expected-hash"]), false).await;
    Mock::given(|request: &wiremock::Request| {
        serde_json::from_slice::<Value>(&request.body)
            .unwrap()
            .is_array()
    })
    .respond_with(ResponseTemplate::new(200).set_body_json(json!([
        {"jsonrpc":"2.0","id":1,"result":transaction("different-hash", 0, Some(RECEIVER), ONE_ETH)}
    ])))
    .expect(1)
    .mount(&server)
    .await;
    let error = evm::block(&node(&server, "base"), 10, &context())
        .await
        .err()
        .unwrap();
    assert!(error.to_string().contains("does not match requested block"));
    server.verify().await;
    server.reset().await;
    mock_block(&server, json!([]), false).await;
    assert!(
        evm::block(&node(&server, "base"), 10, &context())
            .await
            .unwrap()
            .receipts
            .is_empty()
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    server.verify().await;
}

async fn full_block(server: &MockServer, transactions: Value) -> Node {
    Mock::given(matchers::body_partial_json(json!({"method":"eth_getBlockByNumber","params":["0xa",true]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"jsonrpc":"2.0","id":1,"result":{
            "number":"0xa","hash":"block10","parentHash":"block9","timestamp":"0x6553f100","transactions":transactions
        }}))).expect(1).mount(server).await;
    let mut node = node(server, "base");
    node.base_full_blocks = true;
    node
}

#[tokio::test]
async fn base_public_full_block_uses_only_block_and_receipts_with_stable_evidence() {
    let server = MockServer::start().await;
    let direct = transaction("direct", 0, Some(RECEIVER), ONE_ETH);
    let failed = transaction("failed", 1, Some(RECEIVER), ONE_ETH);
    let other = transaction("other", 2, Some(OTHER), ONE_ETH);
    let node = full_block(&server, json!([direct, failed, other])).await;
    mock_receipt(&server, "direct", transaction_receipt(&direct, "0x1")).await;
    mock_receipt(&server, "failed", transaction_receipt(&failed, "0x0")).await;
    let block = evm::block(&node, 10, &context()).await.unwrap();
    assert_eq!(block.receipts.len(), 1);
    assert_eq!(block.receipts[0].tx_hash, "direct");
    assert_eq!(block.receipts[0].amount, bigdecimal::BigDecimal::from(1));
    assert_eq!(block.receipts[0].event_index, 0x8000_0000);
    assert_eq!(block.receipts[0].confirmations, 13);
    assert_eq!(server.received_requests().await.unwrap().len(), 3);
    server.verify().await;
}

#[rstest::rstest]
#[case("hash", Value::Null)]
#[case("blockHash", json!("fork10"))]
#[case("blockNumber", json!("0xb"))]
#[case("transactionIndex", json!("0x1"))]
#[case("to", json!("invalid"))]
#[case("value", json!("no-amount"))]
#[tokio::test]
async fn base_public_full_block_rejects_incomplete_transaction_evidence(
    #[case] field: &str,
    #[case] value: Value,
) {
    let server = MockServer::start().await;
    let mut tx = transaction("direct", 0, Some(RECEIVER), ONE_ETH);
    tx[field] = value;
    let node = full_block(&server, json!([tx])).await;
    assert!(evm::block(&node, 10, &context()).await.is_err());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn base_public_full_block_rejects_hash_only_response_and_wrong_receipt() {
    let server = MockServer::start().await;
    let node = full_block(&server, json!(["tx-hash-only"])).await;
    assert!(evm::block(&node, 10, &context()).await.is_err());
    server.reset().await;
    let tx = transaction("direct", 0, Some(RECEIVER), ONE_ETH);
    let mut receipt = transaction_receipt(&tx, "0x1");
    receipt["blockHash"] = json!("fork10");
    let node = full_block(&server, json!([tx])).await;
    mock_receipt(&server, "direct", receipt).await;
    assert!(evm::block(&node, 10, &context()).await.is_err());
    server.verify().await;
}
