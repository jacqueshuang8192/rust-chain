use super::*;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

#[tokio::test]
async fn all_networks_reach_later_channels_before_retrying_sibling_keys() {
    let Ok(url) = std::env::var("DEPOSIT_MULTICHANNEL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED: isolated DEPOSIT_MULTICHANNEL_TEST_DATABASE_URL required");
        return;
    };
    assert!(url.contains("127.0.0.1") && url.ends_with("timed_deposits_test_multichannel"));
    let pool = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(12)
        .connect(&url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let key = "0123456789abcdef0123456789abcdef";
    for network in ["eth", "base", "tron", "btc", "solana"] {
        sqlx::query("UPDATE wallet_deposit_scanners SET endpoint_encrypted=NULL")
            .execute(&pool)
            .await
            .unwrap();
        let mut servers = Vec::new();
        for priority in 0..6 {
            let server = MockServer::start().await;
            if priority < 5 {
                Mock::given(matchers::method("POST"))
                    .respond_with(ResponseTemplate::new(401))
                    .mount(&server)
                    .await;
            } else {
                Mock::given(matchers::method("POST"))
                    .respond_with(move |request: &wiremock::Request| {
                        healthy_response(network, request)
                    })
                    .mount(&server)
                    .await;
            }
            let encrypted =
                crate::infra::secrets::encrypt_secret_field(key, Some(&server.uri()), None)
                    .unwrap()
                    .unwrap();
            let provider = sqlx::query("INSERT INTO wallet_deposit_scanner_providers(network,name,priority,endpoint_encrypted,request_interval_ms) VALUES (?,'failover fixture',?,?,100)").bind(network).bind(priority).bind(&encrypted).execute(&pool).await.unwrap().last_insert_id();
            for rank in 0..4 {
                sqlx::query("INSERT INTO wallet_deposit_scanner_keys(provider_id,name,priority,request_interval_ms) VALUES (?,'fixture',?,100)").bind(provider).bind(rank).execute(&pool).await.unwrap();
            }
            if priority == 0 {
                sqlx::query("UPDATE wallet_deposit_scanners SET endpoint_encrypted=?,confirmations=2 WHERE network=?").bind(&encrypted).bind(network).execute(&pool).await.unwrap();
            }
            servers.push(server);
        }
        let stream = (network == "base").then_some("native");
        run_group(&pool, Some(key), stream).await.unwrap();
        let snapshot = crate::modules::wallet::application::timed_deposits::settings(&pool)
            .await
            .unwrap();
        let scanner = snapshot
            .scanners
            .iter()
            .find(|s| s.network == network)
            .unwrap();
        let (height, error) = if network == "base" {
            let lane = scanner
                .streams
                .iter()
                .find(|s| s.stream == "native")
                .unwrap();
            (lane.cursor_height, &lane.last_error)
        } else {
            (scanner.cursor_height, &scanner.last_error)
        };
        assert_eq!(height, Some(99), "{network}: {error:?}");
        assert!(error.is_none(), "{network}: {error:?}");
        for server in &servers[..5] {
            assert_eq!(
                server.received_requests().await.unwrap().len(),
                1,
                "{network}: secondary keys starved another provider"
            );
        }
        assert!(
            !servers[5].received_requests().await.unwrap().is_empty(),
            "{network}: sixth provider never attempted"
        );
        for provider in &scanner.providers[..5] {
            assert!(provider.cooldown_until.is_none());
            assert!(provider.keys[0].cooldown_until.is_some());
            assert!(
                provider.keys[1..]
                    .iter()
                    .all(|key| key.cooldown_until.is_none())
            );
        }
    }
    pool.close().await;
}

fn healthy_response(network: &str, request: &wiremock::Request) -> ResponseTemplate {
    let body: Value = serde_json::from_slice(&request.body).unwrap();
    let now = Utc::now().timestamp();
    if network == "tron" {
        let n = body["num"].as_u64().unwrap_or(100);
        let value = if n == 0 {
            json!({"blockID":"genesis2b6653dc"})
        } else {
            json!({"blockID":format!("b{n}"),"block_header":{"raw_data":{"number":n,"parentHash":format!("b{}",n-1),"timestamp":now*1000}}})
        };
        return ResponseTemplate::new(200).set_body_json(value);
    }
    let value = match body["method"].as_str().unwrap() {
        "eth_chainId" => json!(if network == "eth" { "0x1" } else { "0x2105" }),
        "eth_getBlockByNumber" => {
            let n = if body["params"][0] == "latest" {
                100
            } else {
                99
            };
            json!({"number":format!("0x{n:x}"),"hash":format!("b{n}"),"parentHash":format!("b{}",n-1),"timestamp":format!("0x{now:x}")})
        }
        "getblockchaininfo" => {
            json!({"chain":"main","initialblockdownload":false,"headers":100,"blocks":100})
        }
        "getblockhash" => json!("b99"),
        "getblock" => {
            json!({"hash":"b99","height":99,"confirmations":2,"previousblockhash":"b98","time":now})
        }
        "getGenesisHash" => json!("5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d"),
        "getSlot" => json!(100),
        "getBlockTime" => json!(now),
        "getBlocks" => json!([99]),
        "getBlock" => json!({"blockhash":"b99","previousBlockhash":"b98","blockTime":now}),
        other => panic!("unexpected fixture method: {other}"),
    };
    ResponseTemplate::new(200)
        .set_body_json(json!({"jsonrpc":"2.0","id":body["id"],"result":value}))
}

#[tokio::test]
async fn anchor_reads_omit_transaction_payloads_but_payment_reads_require_them() {
    for network in ["btc", "solana"] {
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST"))
            .respond_with(move |r: &wiremock::Request| healthy_response(network, r))
            .mount(&server)
            .await;
        let node = node(&server, network);
        let mut context = Context {
            network: network.into(),
            tip: 100,
            assets: vec![],
            addresses: HashSet::new(),
        };
        let anchor = node.block(99, &context).await.unwrap();
        assert_eq!(anchor.hash, "b99");
        assert!(anchor.receipts.is_empty());
        context
            .assets
            .push(asset("native", "", if network == "btc" { 8 } else { 9 }));
        assert!(
            node.block(99, &context).await.is_err(),
            "anchor-only response must not count as a complete payment scan"
        );
        let bodies: Vec<Value> = server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .map(|r| serde_json::from_slice(&r.body).unwrap())
            .filter(|r: &Value| {
                r["method"]
                    == if network == "btc" {
                        "getblock"
                    } else {
                        "getBlock"
                    }
            })
            .collect();
        assert_eq!(bodies.len(), 2);
        if network == "btc" {
            assert_eq!(bodies[0]["params"][1], 1);
            assert_eq!(bodies[1]["params"][1], 2);
        } else {
            assert_eq!(bodies[0]["params"][1]["transactionDetails"], "none");
            assert_eq!(bodies[1]["params"][1]["transactionDetails"], "full");
        }
    }
}
