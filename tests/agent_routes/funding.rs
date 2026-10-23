use super::*;

const CREATED_MS: i64 = 1_735_732_800_000;
const AMOUNT: &str = "9007199254740993.123456789012345678";
const FEE: &str = "0.000000000000000001";

async fn financial_snapshot(pool: &MySqlPool) -> Result<Vec<Vec<String>>, sqlx::Error> {
    let mut result = Vec::new();
    for (table, fields) in [
        (
            "wallet_deposit_orders",
            "'id',id,'status',status,'paid',CAST(paid_amount AS CHAR)",
        ),
        (
            "quick_recharge_orders",
            "'id',id,'status',status,'amount',CAST(actual_amount AS CHAR)",
        ),
        (
            "wallet_withdrawal_requests",
            "'id',id,'status',status,'amount',CAST(amount AS CHAR)",
        ),
        (
            "wallet_accounts",
            "'id',id,'available',CAST(available AS CHAR),'frozen',CAST(frozen AS CHAR)",
        ),
        ("wallet_ledger", "'id',id,'amount',CAST(amount AS CHAR)"),
        ("platform_financial_journal", "'id',id"),
        ("event_outbox", "'id',id"),
    ] {
        result.push(
            sqlx::query_scalar(&format!(
                "SELECT CAST(JSON_OBJECT({fields}) AS CHAR) FROM {table} ORDER BY id"
            ))
            .fetch_all(pool)
            .await?,
        );
    }
    Ok(result)
}

#[tokio::test]
async fn funding_scope_status_precision_paging_and_no_writes() -> Result<(), Box<dyn Error>> {
    let Some(pool) = mysql_pool().await else {
        return Ok(());
    };
    let root = create_agent(&pool, "fund-root").await;
    let child = create_child_agent(&pool, root, "fund-child").await;
    let grand = create_child_agent(&pool, child, "fund-grand").await;
    let sibling = create_child_agent(&pool, root, "fund-sibling").await;
    let other = create_agent(&pool, "fund-other").await;
    let mut users = Vec::new();
    for owner in [
        Some(root),
        Some(child),
        Some(grand),
        Some(sibling),
        Some(other),
        None,
        Some(grand),
    ] {
        let user = create_user(&pool, "funding").await;
        if let Some(owner) = owner {
            refer_user_to_agent(&pool, user, owner.agent_id, 1).await;
        } else {
            create_unassigned_referral(&pool, user).await;
        }
        users.push(user);
    }
    sqlx::query("UPDATE user_referrals SET direct_inviter_type='user',direct_inviter_id=?,depth=2 WHERE user_id=?").bind(users[2]).bind(users[6]).execute(&pool).await?;
    let asset = create_asset(&pool, "FUND").await;
    let symbol: String = sqlx::query_scalar("SELECT symbol FROM assets WHERE id=?")
        .bind(asset)
        .fetch_one(&pool)
        .await?;
    let address = format!("0x{}00000000", Uuid::now_v7().simple());
    let mapping = sqlx::query("INSERT INTO wallet_deposit_chain_assets (network,asset_id,asset_symbol,asset_kind,token_address,identity_key,decimals,enabled) VALUES ('eth',?,?,'token',?,SHA2(?,256),18,TRUE)")
        .bind(asset).bind(&symbol).bind(&address).bind(&address).execute(&pool).await?.last_insert_id();
    let pool_id = sqlx::query("INSERT INTO wallet_deposit_shared_addresses (network,address,address_key) VALUES ('eth',?,SHA2(?,256))").bind(&address).bind(&address).execute(&pool).await?.last_insert_id();
    let mut chain_ids = Vec::new();
    let mut quick_numbers = Vec::new();
    let mut withdrawal_ids = Vec::new();
    for (i, user) in users.iter().enumerate() {
        let event = sqlx::query("INSERT INTO wallet_deposit_events (user_id,asset_id,asset_symbol,network,address,tx_hash,amount,fee_amount,required_confirmations,status) VALUES (?,?,?,'eth',?,?,?, ?,12,?)")
            .bind(user).bind(asset).bind(&symbol).bind(&address).bind(Uuid::now_v7().to_string()).bind(BigDecimal::from_str(AMOUNT)?).bind(BigDecimal::from_str(FEE)?)
            .bind(if i==2 {"reversed"} else if i==3 {"manual_review"} else {"credited"}).execute(&pool).await?.last_insert_id();
        chain_ids.push(sqlx::query("INSERT INTO wallet_deposit_orders (order_no,user_id,idempotency_key,asset_id,asset_symbol,chain_asset_id,network,asset_kind,token_address,identity_key,decimals,precision_scale,pool_id,address,address_key,amount,fee_amount,required_confirmations,status,deposit_event_id,paid_amount,created_at,expires_at,paid_at) VALUES (?,?,?,?,?,?,'eth','token',?,SHA2(?,256),18,18,?,?,SHA2(?,256),?,?,12,?,?,?,'2025-01-01 12:00:00',?,?)")
            .bind(format!("CD{}",Uuid::now_v7().simple())).bind(user).bind(Uuid::now_v7().to_string()).bind(asset).bind(&symbol).bind(mapping).bind(&address).bind(&address).bind(pool_id).bind(&address).bind(&address)
            .bind(BigDecimal::from_str(AMOUNT)?).bind(BigDecimal::from_str(FEE)?).bind(if i<2 {"pending"}else{"credited"})
            .bind(if i<2 {None}else{Some(event)}).bind(if i<2 {None}else{Some(BigDecimal::from_str(AMOUNT)?)})
            .bind(if i==1 {"2030-01-01 12:30:00"}else{"2025-01-01 12:30:00"}).bind(if i<2 {None}else{Some("2025-01-01 12:15:00")}).execute(&pool).await?.last_insert_id());
        let number = Uuid::now_v7().simple().to_string();
        sqlx::query("INSERT INTO quick_recharge_orders (order_id,user_id,asset_id,asset_symbol,currency,token,network,fiat_amount,actual_amount,status,expiration_time,paid_at,created_at,callback_payload_json,payment_url) VALUES (?,?,?,?,'cny',?,'eth',100,?,'paid',1735734600,'2025-01-01 12:15:00','2025-01-01 12:00:00',JSON_OBJECT('secret','hidden'),'https://private.invalid/payment')")
            .bind(&number).bind(user).bind(asset).bind(&symbol).bind(&symbol).bind(BigDecimal::from_str(AMOUNT)?).execute(&pool).await?;
        quick_numbers.push(number);
        withdrawal_ids.push(sqlx::query("INSERT INTO wallet_withdrawal_requests (user_id,asset_id,asset_symbol,network,address,amount,fee,total_reserved,status,security_method,idempotency_key,gateway_request_id,created_at) VALUES (?,?,?,'eth',?,?,?,?,?,'totp',?,?,'2025-01-01 12:00:00')")
            .bind(user).bind(asset).bind(&symbol).bind(&address).bind(BigDecimal::from_str(AMOUNT)?).bind(BigDecimal::from_str(FEE)?)
            .bind(BigDecimal::from_str("9007199254740993.123456789012345679")?).bind(if i==2 {"unknown_broadcast"}else{"pending_review"}).bind(Uuid::now_v7().to_string()).bind(Uuid::now_v7().to_string()).execute(&pool).await?.last_insert_id());
    }
    let settings = test_settings();
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
    let before = financial_snapshot(&pool).await?;
    for (kind, multiplier) in [("deposits", 2), ("withdrawals", 1)] {
        let path = format!("/agent/api/v1/funding/{kind}");
        for (token, expected) in [
            (&root_token, vec![0, 1, 2, 3, 6]),
            (&child_token, vec![1, 2, 6]),
            (&grand_token, vec![2, 6]),
        ] {
            let (status, payload) = agent_get_json(app.clone(), token, &path).await?;
            assert_eq!(status, StatusCode::OK, "{kind}: {payload}");
            assert_eq!(payload["total"], expected.len() * multiplier);
            let rows = payload["items"].as_array().unwrap();
            assert_eq!(rows.len(), expected.len() * multiplier);
            for row in rows {
                assert!(expected.iter().any(|i| row["user_id"] == users[*i]));
                assert_eq!(row["created_at"], CREATED_MS);
                assert_eq!(row["amount"], AMOUNT);
                assert!(row["owner_agent_id"].is_number());
                assert!(row["owner_agent_code"].is_string());
                for private in [
                    "security_method",
                    "idempotency_key",
                    "gateway_request_id",
                    "callback_payload_json",
                    "payment_url",
                    "review_reason",
                    "reviewed_by",
                ] {
                    assert!(row.get(private).is_none(), "leaked {private}");
                }
            }
        }
        for (query, count) in [
            ("team_scope=own".to_owned(), 1),
            ("team_scope=descendants".to_owned(), 4),
            (format!("agent_id={}", child.agent_id), 3),
            (format!("user_id={}", users[2]), 1),
            (format!("user_id={}", users[4]), 0),
            (format!("user_id={}", users[5]), 0),
            ("user_id=9007199254740991".to_owned(), 0),
            ("email=%25".to_owned(), 0),
            (
                format!("created_from={CREATED_MS}&created_to={}", CREATED_MS + 1),
                5,
            ),
            (format!("created_to={CREATED_MS}"), 0),
            (
                format!("asset_symbol={}&network=ETH", symbol.to_lowercase()),
                5,
            ),
        ] {
            let (status, payload) =
                agent_get_json(app.clone(), &root_token, format!("{path}?{query}")).await?;
            assert_eq!(status, StatusCode::OK, "{query}: {payload}");
            assert_eq!(payload["total"], count * multiplier, "{kind} {query}");
            assert_eq!(
                payload["items"].as_array().unwrap().len(),
                count * multiplier
            );
        }
        for agent in [root, sibling, other] {
            let (_, payload) = agent_get_json(
                app.clone(),
                &child_token,
                format!("{path}?agent_id={}", agent.agent_id),
            )
            .await?;
            assert_eq!(payload["total"], 0);
            assert_eq!(payload["items"], json!([]));
        }
        for user in [users[0], users[3], users[4], users[5]] {
            let (_, payload) =
                agent_get_json(app.clone(), &child_token, format!("{path}?user_id={user}")).await?;
            assert_eq!(payload["total"], 0);
        }
        let (_, all) = agent_get_json(app.clone(), &root_token, &path).await?;
        let (_, page) =
            agent_get_json(app.clone(), &root_token, format!("{path}?limit=2&offset=1")).await?;
        assert_eq!(page["total"], all["total"]);
        assert_eq!(page["items"], json!(all["items"].as_array().unwrap()[1..3]));
    }
    for (query, count) in [
        ("source=quick", 5),
        ("source=chain", 5),
        ("source=chain&status=expired", 1),
        ("source=chain&status=pending", 1),
        ("source=chain&status=reversed", 1),
        ("source=chain&status=review", 1),
        ("source=chain&status=credited", 1),
        ("status=credited", 6),
    ] {
        let (status, payload) = agent_get_json(
            app.clone(),
            &root_token,
            format!("/agent/api/v1/funding/deposits?{query}"),
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "{payload}");
        assert_eq!(payload["total"], count, "{query}");
    }
    let (_, chain) = agent_get_json(
        app.clone(),
        &root_token,
        format!(
            "/agent/api/v1/funding/deposits?source=chain&record_id={}",
            chain_ids[6]
        ),
    )
    .await?;
    assert_eq!(
        chain["items"][0]["credited_amount"],
        "9007199254740993.123456789012345677"
    );
    assert_eq!(
        BigDecimal::from_str(chain["items"][0]["fee"].as_str().unwrap())?,
        BigDecimal::from_str(FEE)?
    );
    let (_, quick) = agent_get_json(
        app.clone(),
        &root_token,
        format!(
            "/agent/api/v1/funding/deposits?order_no={}",
            quick_numbers[0]
        ),
    )
    .await?;
    assert_eq!(quick["total"], 1);
    assert_eq!(quick["items"][0]["credited_amount"], AMOUNT);
    assert_eq!(quick["items"][0]["fiat_currency"], "CNY");
    assert_eq!(
        BigDecimal::from_str(quick["items"][0]["fiat_amount"].as_str().unwrap())?,
        BigDecimal::from(100)
    );
    let (_, withdraw) = agent_get_json(
        app.clone(),
        &root_token,
        format!(
            "/agent/api/v1/funding/withdrawals?record_id={}&status=unknown_broadcast",
            withdrawal_ids[2]
        ),
    )
    .await?;
    assert_eq!(withdraw["total"], 1);
    assert_eq!(
        withdraw["items"][0]["total_reserved"],
        "9007199254740993.123456789012345679"
    );
    assert_eq!(financial_snapshot(&pool).await?, before);
    // 同一身份查询后用户改归属，下一次列表和计数都必须立即使用新的子树关系。
    sqlx::query("UPDATE user_referrals SET root_agent_id=? WHERE user_id=?")
        .bind(other.agent_id)
        .bind(users[6])
        .execute(&pool)
        .await?;
    for kind in ["deposits", "withdrawals"] {
        let (_, payload) = agent_get_json(
            app.clone(),
            &grand_token,
            format!("/agent/api/v1/funding/{kind}?user_id={}", users[6]),
        )
        .await?;
        assert_eq!(payload["total"], 0);
        assert_eq!(payload["items"], json!([]));
    }
    sqlx::query("UPDATE agents SET status='disabled' WHERE id=?")
        .bind(child.agent_id)
        .execute(&pool)
        .await?;
    for kind in ["deposits", "withdrawals"] {
        let (status, _) = agent_get_json(
            app.clone(),
            &grand_token,
            format!("/agent/api/v1/funding/{kind}"),
        )
        .await?;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    for user in &users {
        for table in [
            "wallet_deposit_orders",
            "wallet_deposit_events",
            "quick_recharge_orders",
            "wallet_withdrawal_requests",
        ] {
            sqlx::query(&format!("DELETE FROM {table} WHERE user_id=?"))
                .bind(user)
                .execute(&pool)
                .await?;
        }
    }
    sqlx::query("DELETE FROM wallet_deposit_chain_assets WHERE id=?")
        .bind(mapping)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM wallet_deposit_shared_addresses WHERE pool_id=?")
        .bind(pool_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM assets WHERE id=?")
        .bind(asset)
        .execute(&pool)
        .await?;
    cleanup_agent_fixture(&pool, &[grand, sibling, child, other, root], &users).await?;
    Ok(())
}

#[tokio::test]
async fn funding_auth_and_validation_precede_database_access() -> Result<(), Box<dyn Error>> {
    let settings = test_settings();
    let token = issue_token(&settings, "agent:1".to_owned(), TokenScope::Agent, 900)?;
    let user = issue_token(&settings, "user:1".to_owned(), TokenScope::User, 900)?;
    let app = build_router(AppState::new(settings));
    for kind in ["deposits", "withdrawals"] {
        let path = format!("/agent/api/v1/funding/{kind}");
        let response = app
            .clone()
            .oneshot(Request::builder().uri(&path).body(Body::empty())?)
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let (status, _) = agent_get_json(app.clone(), &user, &path).await?;
        assert_eq!(status, StatusCode::FORBIDDEN);
        for query in [
            "status=bogus",
            "team_scope=bogus",
            "user_id=0",
            "agent_id=9007199254740992",
            "record_id=-1",
            "user_id=1.5",
            "limit=0",
            "limit=101",
            "offset=100001",
            "created_from=-1",
            "created_to=253402300800000",
            "created_from=10&created_to=10",
            "source=unknown",
            "path=/agent:1",
            "agent_id=1&agent_id=2",
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("{path}?{query}"))
                        .header("authorization", format!("Bearer {token}"))
                        .body(Body::empty())?,
                )
                .await?;
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{kind}?{query}");
        }
    }
    for path in [
        "withdrawals?source=quick",
        "withdrawals?order_no=x",
        "withdrawals?status=pending",
    ] {
        let (status, _) =
            agent_get_json(app.clone(), &token, format!("/agent/api/v1/funding/{path}")).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (status, _) = agent_get_json(app, &token, "/agent/api/v1/funding/unknown").await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    Ok(())
}
