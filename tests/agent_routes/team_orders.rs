use super::*;

const KINDS: [&str; 6] = [
    "seconds",
    "margin",
    "earn",
    "spot",
    "new-coin-subscriptions",
    "new-coin-purchases",
];
const CREATED_MS: i64 = 1_735_732_800_000;

async fn seed_orders(
    pool: &MySqlPool,
    user: u64,
    pair: u64,
    asset: u64,
    products: [u64; 4],
) -> Result<[u64; 6], sqlx::Error> {
    let seconds = sqlx::query(
        "INSERT INTO seconds_contract_orders \
         (user_id, product_id, pair_id, stake_asset, direction, stake_amount, duration_seconds, \
         payout_rate, entry_price, status, idempotency_key, expires_at, created_at) \
         VALUES (?, ?, ?, ?, 'up', 7.123456789012345678, 60, 0.8, 100, 'opened', ?, \
         '2025-01-01 12:01:00', '2025-01-01 12:00:00')",
    )
    .bind(user)
    .bind(products[0])
    .bind(pair)
    .bind(asset)
    .bind(Uuid::now_v7().to_string())
    .execute(pool)
    .await?
    .last_insert_id();
    let margin = sqlx::query(
        "INSERT INTO margin_positions \
         (user_id, product_id, pair_id, margin_asset, wallet_scope, margin_mode, direction, \
         order_type, margin_amount, leverage, notional_amount, borrowed_amount, interest_amount, \
         entry_price, limit_price, status, idempotency_key, created_at) \
         VALUES (?, ?, ?, ?, 'margin', 'isolated', 'long', 'limit', 10, 2, 20, 10, 0, \
         NULL, 90.123456789012345678, 'opened', ?, '2025-01-01 12:00:00')",
    )
    .bind(user)
    .bind(products[1])
    .bind(pair)
    .bind(asset)
    .bind(Uuid::now_v7().to_string())
    .execute(pool)
    .await?
    .last_insert_id();
    let earn = sqlx::query(
        "INSERT INTO earn_subscriptions \
         (user_id, product_id, asset_id, amount, apr_rate, term_days, status, idempotency_key, \
         subscribed_at, matures_at) \
         VALUES (?, ?, ?, 9007199254740993.123456789012345678, 0.08, 30, 'subscribed', ?, \
         '2025-01-01 12:00:00', '2025-01-31 12:00:00')",
    )
    .bind(user)
    .bind(products[2])
    .bind(asset)
    .bind(Uuid::now_v7().to_string())
    .execute(pool)
    .await?
    .last_insert_id();
    let spot = sqlx::query(
        "INSERT INTO spot_orders \
         (user_id, pair_id, side, order_type, price, quantity, filled_quantity, status, created_at) \
         VALUES (?, ?, 'buy', 'limit', 90.123456789012345678, 2, 1, 'partially_filled', '2025-01-01 12:00:00')",
    ).bind(user).bind(pair).execute(pool).await?.last_insert_id();
    let subscription = sqlx::query("INSERT INTO new_coin_subscriptions (project_id,user_id,quote_asset,issue_price,quote_amount,requested_quantity,allocated_quantity,status,idempotency_key,created_at) VALUES (?,?,?,2.5,25,10,10,'available',?,'2025-01-01 12:00:00')")
        .bind(products[3]).bind(user).bind(asset).bind(Uuid::now_v7().to_string()).execute(pool).await?.last_insert_id();
    let purchase = sqlx::query("INSERT INTO new_coin_purchase_orders (project_id,user_id,pair_id,base_asset,quote_asset,price,quantity,quote_amount,status,idempotency_key,created_at) SELECT ?,?,?,base_asset,?,2.5,10,25,'available',?,'2025-01-01 12:00:00' FROM trading_pairs WHERE id=?")
        .bind(products[3]).bind(user).bind(pair).bind(asset).bind(Uuid::now_v7().to_string()).bind(pair).execute(pool).await?.last_insert_id();
    Ok([seconds, margin, earn, spot, subscription, purchase])
}

async fn read_snapshot(pool: &MySqlPool, user: u64) -> Result<Vec<Vec<String>>, sqlx::Error> {
    let mut snapshot = Vec::new();
    for (table, fields) in [
        (
            "new_coin_subscriptions",
            "'id',id,'status',status,'quote',CAST(quote_amount AS CHAR),'allocated',CAST(allocated_quantity AS CHAR),'frozen',CAST(frozen_quote_amount AS CHAR),'settled',CAST(settled_quote_amount AS CHAR),'refund',CAST(refunded_quote_amount AS CHAR)",
        ),
        (
            "new_coin_purchase_orders",
            "'id',id,'status',status,'quote',CAST(quote_amount AS CHAR),'quantity',CAST(quantity AS CHAR),'lock',lock_position_id",
        ),
        (
            "seconds_contract_orders",
            "'id',id,'status',status,'amount',CAST(stake_amount AS CHAR),'price',CAST(settlement_price AS CHAR),'settled',settled_at",
        ),
        (
            "margin_positions",
            "'id',id,'status',status,'margin',CAST(margin_amount AS CHAR),'interest',CAST(interest_amount AS CHAR),'entry',CAST(entry_price AS CHAR),'closed',closed_at",
        ),
        (
            "earn_subscriptions",
            "'id',id,'status',status,'amount',CAST(amount AS CHAR),'redeemed',redeemed_at",
        ),
        (
            "spot_orders",
            "'id',id,'status',status,'quantity',CAST(quantity AS CHAR),'filled',CAST(filled_quantity AS CHAR)",
        ),
        (
            "wallet_accounts",
            "'id',id,'available',CAST(available AS CHAR),'frozen',CAST(frozen AS CHAR),'locked',CAST(locked AS CHAR)",
        ),
        (
            "margin_wallet_accounts",
            "'id',id,'available',CAST(available AS CHAR),'frozen',CAST(frozen AS CHAR),'locked',CAST(locked AS CHAR)",
        ),
    ] {
        snapshot.push(sqlx::query_scalar(&format!(
            "SELECT CAST(JSON_OBJECT({fields}) AS CHAR) FROM {table} WHERE user_id = ? ORDER BY id"
        )).bind(user).fetch_all(pool).await?);
    }
    Ok(snapshot)
}

#[tokio::test]
async fn team_orders_scope_filters_paging_precision_and_read_only() -> Result<(), Box<dyn Error>> {
    let Some(pool) = mysql_pool().await else {
        return Ok(());
    };
    let settings = test_settings();
    let root = create_agent(&pool, "to-root").await;
    let child = create_child_agent(&pool, root, "to-child").await;
    let grand = create_child_agent(&pool, child, "to-grand").await;
    let sibling = create_child_agent(&pool, root, "to-sibling").await;
    let other = create_agent(&pool, "to-other").await;
    let mut users = Vec::new();
    let mut owners = Vec::new();
    for owner in [
        Some(root),
        Some(child),
        Some(grand),
        Some(sibling),
        Some(other),
        None,
        Some(grand),
    ] {
        let user = create_user(&pool, "team-orders").await;
        if let Some(owner) = owner {
            refer_user_to_agent(&pool, user, owner.agent_id, 1).await;
        } else {
            create_unassigned_referral(&pool, user).await;
        }
        users.push(user);
        owners.push(owner.map(|agent| agent.agent_id));
    }
    // 用户邀请的下级用户继承原归属代理，不能漏出团队订单列表。
    sqlx::query("UPDATE user_referrals SET direct_inviter_type = 'user', direct_inviter_id = ?, depth = 2 WHERE user_id = ?")
        .bind(users[2]).bind(users[6]).execute(&pool).await?;
    let base = create_asset(&pool, "tobase").await;
    let quote = create_asset(&pool, "toquote").await;
    let symbol = format!("TO{}-USD", Uuid::now_v7().simple());
    let pair = sqlx::query("INSERT INTO trading_pairs (base_asset, quote_asset, symbol, price_precision, qty_precision, min_order_value, status, market_type) VALUES (?, ?, ?, 18, 18, 1, 'active', 'external')")
        .bind(base).bind(quote).bind(&symbol).execute(&pool).await?.last_insert_id();
    let seconds_product = sqlx::query("INSERT INTO seconds_contract_products (pair_id, stake_asset, duration_seconds, payout_rate, min_stake, max_stake, status) VALUES (?, ?, 60, 0.8, 1, 1000, 'active')")
        .bind(pair).bind(quote).execute(&pool).await?.last_insert_id();
    let margin_product = sqlx::query("INSERT INTO margin_products (pair_id, margin_asset, margin_mode, margin_modes, leverage_levels, max_leverage, min_margin, max_margin, maintenance_margin_rate, status) VALUES (?, ?, 'isolated', JSON_ARRAY('isolated'), JSON_ARRAY('2'), 5, 1, 1000, 0.05, 'active')")
        .bind(pair).bind(quote).execute(&pool).await?.last_insert_id();
    let earn_product = sqlx::query("INSERT INTO earn_products (asset_id, name, introduction_json, term_days, apr_rate, min_subscribe, status) VALUES (?, 'Team Earn', JSON_OBJECT(), 30, 0.08, 1, 'active')")
        .bind(quote).execute(&pool).await?.last_insert_id();
    let project_symbol = format!("NC{}", Uuid::now_v7().simple());
    let project = sqlx::query("INSERT INTO new_coin_projects (asset_id,symbol,lifecycle_status,total_supply,issue_price,quote_asset_id,reserved_supply,allocated_supply,remaining_supply,unlock_type,fixed_unlock_at,status) VALUES (?,?,'listed',1000000,2.5,?,0,0,1000000,'fixed_time','2030-01-01','active')")
        .bind(base).bind(&project_symbol).bind(quote).execute(&pool).await?.last_insert_id();
    let products = [seconds_product, margin_product, earn_product, project];
    let mut ids = Vec::new();
    for user in &users {
        ids.push(seed_orders(&pool, *user, pair, quote, products).await?);
    }
    sqlx::query("UPDATE seconds_contract_orders SET status = 'refunded' WHERE id = ?")
        .bind(ids[6][0])
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE margin_positions SET entry_price = 100 WHERE id = ?")
        .bind(ids[6][1])
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE earn_subscriptions SET status = 'redeemed', redeemed_at = '2025-01-31 12:00:00' WHERE id = ?")
        .bind(ids[6][2]).execute(&pool).await?;
    sqlx::query("UPDATE spot_orders SET status = 'filled', filled_quantity = 2 WHERE id = ?")
        .bind(ids[6][3])
        .execute(&pool)
        .await?;
    sqlx::query("UPDATE new_coin_subscriptions SET settlement_mode='manual_distribution',status='partial_allocated',allocated_quantity=4,settled_quote_amount=10,refunded_quote_amount=15 WHERE id=?")
        .bind(ids[6][4]).execute(&pool).await?;
    sqlx::query("UPDATE new_coin_purchase_orders SET status='pending' WHERE id=?")
        .bind(ids[6][5])
        .execute(&pool)
        .await?;
    for table in ["wallet_accounts", "margin_wallet_accounts"] {
        sqlx::query(&format!("INSERT INTO {table} (user_id, asset_id, available, frozen, locked) VALUES (?, ?, 12.123456789012345678, 2, 3)"))
            .bind(users[2]).bind(quote).execute(&pool).await?;
    }
    let mut before = Vec::new();
    for user in &users {
        before.push(read_snapshot(&pool, *user).await?);
    }
    let evidence_sql = "SELECT (SELECT COUNT(*) FROM wallet_ledger), (SELECT COUNT(*) FROM event_outbox), (SELECT COUNT(*) FROM platform_financial_journal)";
    let evidence_before: (i64, i64, i64) = sqlx::query_as(evidence_sql).fetch_one(&pool).await?;
    let token = |agent: AgentFixture| {
        issue_token(
            &settings,
            format!("agent:{}", agent.admin_user_id),
            TokenScope::Agent,
            900,
        )
        .unwrap()
    };
    let root_token = token(root);
    let child_token = token(child);
    let grand_token = token(grand);
    let app = build_router(AppState::new(settings).with_mysql(pool.clone()));

    for (index, kind) in KINDS.iter().enumerate() {
        let path = format!("/agent/api/v1/orders/{kind}");
        for (token, expected) in [
            (&root_token, vec![0, 1, 2, 3, 6]),
            (&child_token, vec![1, 2, 6]),
            (&grand_token, vec![2, 6]),
        ] {
            let (status, payload) = agent_get_json(app.clone(), token, &path).await?;
            assert_eq!(status, StatusCode::OK, "{kind}: {payload}");
            assert_eq!(payload["total"], expected.len());
            let rows = payload["orders"].as_array().unwrap();
            let actual: Vec<u64> = rows.iter().map(|row| row["id"].as_u64().unwrap()).collect();
            let mut expected_ids: Vec<u64> = expected.iter().map(|i| ids[*i][index]).collect();
            expected_ids.sort_by(|a, b| b.cmp(a));
            assert_eq!(actual, expected_ids);
            for row in rows {
                let user_index = users
                    .iter()
                    .position(|id| Some(*id) == row["user_id"].as_u64())
                    .unwrap();
                assert_eq!(row["owner_agent_id"].as_u64(), owners[user_index]);
                assert!(row["owner_agent_code"].is_string());
                assert!(row["email"].is_string());
                assert_eq!(row["created_at"], CREATED_MS);
            }
        }
        for (query, expected) in [
            ("team_scope=own".to_owned(), 1),
            ("team_scope=descendants".to_owned(), 4),
            (format!("agent_id={}", child.agent_id), 3),
            (format!("agent_id={}", grand.agent_id), 2),
            (format!("user_id={}", users[2]), 1),
            (format!("order_id={}", ids[2][index]), 1),
            (format!("user_id={}", users[4]), 0),
            ("user_id=9007199254740991".to_owned(), 0),
            ("email=%25".to_owned(), 0),
            ("email=team-orders".to_owned(), 5),
            (
                format!("created_from={CREATED_MS}&created_to={}", CREATED_MS + 1),
                5,
            ),
            (format!("created_to={CREATED_MS}"), 0),
            (format!("created_from={}", CREATED_MS + 1), 0),
        ] {
            let (status, payload) =
                agent_get_json(app.clone(), &root_token, format!("{path}?{query}")).await?;
            assert_eq!(status, StatusCode::OK, "{kind} {query}: {payload}");
            assert_eq!(payload["total"], expected, "{kind} {query}");
            assert_eq!(
                payload["orders"].as_array().unwrap().len(),
                expected as usize
            );
        }
        for agent in [root, sibling, other] {
            let (_, payload) = agent_get_json(
                app.clone(),
                &child_token,
                format!("{path}?agent_id={}", agent.agent_id),
            )
            .await?;
            assert_eq!(payload, json!({"orders": [], "total": 0}));
        }
        for (offset, expected_id) in [(0, ids[6][index]), (1, ids[3][index])] {
            let (_, payload) = agent_get_json(
                app.clone(),
                &root_token,
                format!("{path}?limit=1&offset={offset}"),
            )
            .await?;
            assert_eq!(payload["total"], 5);
            assert_eq!(payload["orders"].as_array().unwrap().len(), 1);
            assert_eq!(payload["orders"][0]["id"], expected_id);
        }
        let (_, empty) = agent_get_json(
            app.clone(),
            &root_token,
            format!("{path}?limit=1&offset=100000"),
        )
        .await?;
        assert_eq!(empty, json!({"orders": [], "total": 5}));
        let (status_filter, expected_id) = [
            ("refunded", ids[6][0]),
            ("opened", ids[6][1]),
            ("redeemed", ids[6][2]),
            ("filled", ids[6][3]),
            ("partial_allocated", ids[6][4]),
            ("pending", ids[6][5]),
        ][index];
        let (_, payload) = agent_get_json(
            app.clone(),
            &root_token,
            format!("{path}?status={status_filter}"),
        )
        .await?;
        assert_eq!(payload["total"], 1);
        assert_eq!(payload["orders"][0]["id"], expected_id);
        if index >= 4 {
            let (_, payload) = agent_get_json(
                app.clone(),
                &root_token,
                format!(
                    "{path}?project_id={project}&symbol={}",
                    project_symbol.to_lowercase()
                ),
            )
            .await?;
            assert_eq!(payload["total"], 5);
            for row in payload["orders"].as_array().unwrap() {
                assert!(row.get("idempotency_key").is_none());
                assert!(row["quote_symbol"].is_string());
                assert_eq!(row["quote_amount"], "25.000000000000000000");
                if index == 4 && row["status"] == "available" {
                    assert!(row["settled_quote_amount"].is_null());
                    assert!(row["refunded_quote_amount"].is_null());
                }
            }
            for query in ["project_id=9007199254740991", "symbol=missing"] {
                let (_, payload) =
                    agent_get_json(app.clone(), &root_token, format!("{path}?{query}")).await?;
                assert_eq!(payload, json!({"orders": [], "total": 0}));
            }
        }
        if index < 3 {
            let (_, payload) = agent_get_json(
                app.clone(),
                &root_token,
                format!("{path}?product_id={}", products[index]),
            )
            .await?;
            assert_eq!(payload["total"], 5);
        }
        if index < 4 && *kind != "earn" {
            let direction = ["up", "long", "", "buy"][index];
            let (_, payload) = agent_get_json(
                app.clone(),
                &root_token,
                format!(
                    "{path}?symbol={}&direction={direction}&user_id={}",
                    symbol.replace('-', "_").to_lowercase(),
                    users[2]
                ),
            )
            .await?;
            assert_eq!(payload["total"], 1);
        }
    }
    for (kind, field, expected) in [
        ("seconds", "stake_amount", "7.123456789012345678"),
        ("margin", "limit_price", "90.123456789012345678"),
        ("earn", "amount", "9007199254740993.123456789012345678"),
        ("spot", "price", "90.123456789012345678"),
    ] {
        let (_, payload) = agent_get_json(
            app.clone(),
            &root_token,
            format!("/agent/api/v1/orders/{kind}?user_id={}", users[2]),
        )
        .await?;
        assert_eq!(payload["orders"][0][field], expected);
    }
    let (_, pending) = agent_get_json(
        app.clone(),
        &root_token,
        "/agent/api/v1/orders/margin?status=pending",
    )
    .await?;
    assert_eq!(pending["total"], 4);
    assert!(
        pending["orders"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["entry_price"].is_null() && row["status"] == "opened")
    );
    let (_, no_result) = agent_get_json(
        app.clone(),
        &root_token,
        "/agent/api/v1/orders/seconds?result=win",
    )
    .await?;
    assert_eq!(no_result["total"], 0);
    for (i, user) in users.iter().enumerate() {
        assert_eq!(read_snapshot(&pool, *user).await?, before[i]);
    }
    let evidence_after: (i64, i64, i64) = sqlx::query_as(evidence_sql).fetch_one(&pool).await?;
    assert_eq!(evidence_before, evidence_after);
    sqlx::query("UPDATE agents SET status = 'disabled' WHERE id = ?")
        .bind(child.agent_id)
        .execute(&pool)
        .await?;
    for kind in KINDS {
        let (status, _) = agent_get_json(
            app.clone(),
            &grand_token,
            format!("/agent/api/v1/orders/{kind}"),
        )
        .await?;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    for user in &users {
        for table in [
            "seconds_contract_orders",
            "margin_positions",
            "earn_subscriptions",
            "spot_orders",
            "new_coin_subscriptions",
            "new_coin_purchase_orders",
            "wallet_accounts",
            "margin_wallet_accounts",
        ] {
            sqlx::query(&format!("DELETE FROM {table} WHERE user_id = ?"))
                .bind(user)
                .execute(&pool)
                .await?;
        }
    }
    for (table, id) in [
        ("seconds_contract_products", seconds_product),
        ("margin_products", margin_product),
        ("earn_products", earn_product),
        ("new_coin_projects", project),
        ("trading_pairs", pair),
        ("assets", base),
        ("assets", quote),
    ] {
        sqlx::query(&format!("DELETE FROM {table} WHERE id = ?"))
            .bind(id)
            .execute(&pool)
            .await?;
    }
    cleanup_agent_fixture(&pool, &[grand, sibling, child, other, root], &users).await?;
    Ok(())
}

#[tokio::test]
async fn team_orders_reject_invalid_filters_before_database_and_require_agent_auth()
-> Result<(), Box<dyn Error>> {
    let settings = test_settings();
    let token = issue_token(&settings, "agent:1".to_owned(), TokenScope::Agent, 900)?;
    let user_token = issue_token(&settings, "user:1".to_owned(), TokenScope::User, 900)?;
    let app = build_router(AppState::new(settings));
    for kind in KINDS {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/agent/api/v1/orders/{kind}"))
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let (status, _) = agent_get_json(
            app.clone(),
            &user_token,
            format!("/agent/api/v1/orders/{kind}"),
        )
        .await?;
        assert_eq!(status, StatusCode::FORBIDDEN);
        for query in [
            "status=bogus",
            "team_scope=bogus",
            "user_id=0",
            "agent_id=9007199254740992",
            "limit=0",
            "limit=101",
            "offset=100001",
            "order_id=-1",
            "user_id=1.5",
            "created_from=-1",
            "created_to=253402300800000",
            "created_from=10&created_to=10",
            "direction=invalid",
            "result=invalid",
            "path=/agent:1",
            "agent_id=1&agent_id=2",
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/agent/api/v1/orders/{kind}?{query}"))
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{kind}?{query}");
        }
    }
    for path in [
        "earn?symbol=BTC",
        "earn?direction=long",
        "margin?result=win",
        "spot?product_id=1",
        "earn?project_id=1",
        "new-coin-subscriptions?product_id=1",
        "new-coin-subscriptions?direction=buy",
        "new-coin-subscriptions?project_id=0",
        "new-coin-purchases?result=win",
        "new-coin-purchases?project_id=9007199254740992",
    ] {
        let (status, _) =
            agent_get_json(app.clone(), &token, format!("/agent/api/v1/orders/{path}")).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (status, _) = agent_get_json(app, &token, "/agent/api/v1/orders/unknown").await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    Ok(())
}

#[tokio::test]
async fn team_orders_openapi_documents_scoped_filters_and_business_snapshots()
-> Result<(), Box<dyn Error>> {
    let response = build_router(AppState::new(test_settings()))
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())?,
        )
        .await?;
    let bytes = axum::body::to_bytes(response.into_body(), 4_000_000).await?;
    let doc: Value = serde_json::from_slice(&bytes)?;
    let operation = &doc["paths"]["/agent/api/v1/orders/{kind}"]["get"];
    assert!(operation["security"][0]["bearerAuth"].is_array());
    let names: Vec<&str> = operation["parameters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|parameter| parameter["name"].as_str().unwrap())
        .collect();
    for name in [
        "kind",
        "team_scope",
        "agent_id",
        "user_id",
        "email",
        "order_id",
        "status",
        "product_id",
        "project_id",
        "symbol",
        "direction",
        "result",
        "created_from",
        "created_to",
        "limit",
        "offset",
    ] {
        assert!(names.contains(&name), "{name}");
    }
    let schemas = &doc["components"]["schemas"];
    fn verify_refs(value: &Value, schemas: &Value) {
        match value {
            Value::Object(map) => {
                if let Some(reference) = map.get("$ref").and_then(Value::as_str)
                    && let Some(name) = reference.strip_prefix("#/components/schemas/")
                {
                    assert!(!schemas[name].is_null(), "missing schema {name}");
                }
                for nested in map.values() {
                    verify_refs(nested, schemas);
                }
            }
            Value::Array(values) => {
                for value in values {
                    verify_refs(value, schemas);
                }
            }
            _ => {}
        }
    }
    verify_refs(&schemas["AgentTeamOrdersResponse"], schemas);
    assert_eq!(
        schemas["AgentTeamEarnOrder"]["properties"]["amount"]["type"],
        "string"
    );
    assert_eq!(
        schemas["AgentTeamEarnOrder"]["properties"]["created_at"]["format"],
        "int64"
    );
    assert!(
        schemas["AgentUserSecondsContractOrderResponse"]["properties"]["status"]["pattern"]
            .as_str()
            .unwrap()
            .contains("refunded")
    );
    for name in ["AgentTeamNewCoinSubscription", "AgentTeamNewCoinPurchase"] {
        assert_eq!(
            schemas[name]["properties"]["quote_amount"]["type"],
            "string"
        );
        assert_eq!(schemas[name]["properties"]["created_at"]["format"], "int64");
        assert!(schemas[name]["properties"].get("idempotency_key").is_none());
    }
    assert!(
        schemas["AgentTeamNewCoinSubscription"]["properties"]["status"]["pattern"]
            .as_str()
            .unwrap()
            .contains("available")
    );
    assert!(
        schemas["AgentTeamNewCoinSubscription"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!("settled_quote_amount"))
    );
    // 包络不能退化为无类型 object，各业务 DTO 都要能沿引用解析。
    let encoded = serde_json::to_string(&schemas["AgentTeamOrdersResponse"])?;
    for kind in [
        "Seconds",
        "Margin",
        "Earn",
        "Spot",
        "NewCoinSubscription",
        "NewCoinPurchase",
    ] {
        assert!(encoded.contains(kind), "{encoded}");
    }
    Ok(())
}
