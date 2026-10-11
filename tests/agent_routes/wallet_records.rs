use super::*;

fn token(agent: AgentFixture) -> String {
    issue_token(
        &test_settings(),
        format!("agent:{}", agent.admin_user_id),
        TokenScope::Agent,
        900,
    )
    .unwrap()
}

#[tokio::test]
async fn wallet_records_reject_wrong_auth_and_filters_before_database_access()
-> Result<(), Box<dyn Error>> {
    let settings = test_settings();
    let app = build_router(AppState::new(settings.clone()));
    let agent = issue_token(&settings, "agent:1", TokenScope::Agent, 900)?;
    let user = issue_token(&settings, "user:1", TokenScope::User, 900)?;
    let path = "/agent/api/v1/users/1/wallet-ledger";
    assert_eq!(
        agent_get_json(app.clone(), &user, path).await?.0,
        StatusCode::FORBIDDEN
    );
    for query in [
        "account_type=all",
        "account_type=spot%27",
        "balance_type=total",
    ] {
        assert_eq!(
            agent_get_json(app.clone(), &agent, format!("{path}?{query}"))
                .await?
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    let response = app
        .oneshot(Request::builder().uri(path).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[tokio::test]
async fn wallet_records_scoped_readonly_queries_and_admin_account_selection()
-> Result<(), Box<dyn Error>> {
    let Some(pool) = mysql_pool().await else {
        return Ok(());
    };
    let root = create_agent(&pool, "wr-root").await;
    let child = create_child_agent(&pool, root, "wr-child").await;
    let sibling = create_child_agent(&pool, root, "wr-sibling").await;
    let other = create_agent(&pool, "wr-other").await;
    let user = create_user(&pool, "wr-user").await;
    let outside = create_user(&pool, "wr-outside").await;
    refer_user_to_agent(&pool, user, child.agent_id, 1).await;
    refer_user_to_agent(&pool, outside, root.agent_id, 1).await;
    let asset = create_asset(&pool, "wr-asset").await;
    for (table, change, amount) in [
        (
            "wallet_ledger",
            "deposit",
            "9007199254740993.000000000000000001",
        ),
        (
            "wallet_ledger",
            "seconds_contract_settle_win",
            "0.000000000000000001",
        ),
        (
            "margin_wallet_ledger",
            "margin_close",
            "-0.000000000000000001",
        ),
    ] {
        sqlx::query(&format!("INSERT INTO {table} (user_id,asset_id,change_type,amount,balance_type,balance_after,available_after,frozen_after,locked_after,ref_type,ref_id) VALUES (?, ?, ?, ?, 'available', ?, ?, 2, 3, 'test', 'ref-1')"))
            .bind(user).bind(asset).bind(change).bind(BigDecimal::from_str(amount)?)
            .bind(BigDecimal::from_str("9007199254740993.000000000000000001")?)
            .bind(BigDecimal::from_str("9007199254740993.000000000000000001")?)
            .execute(&pool).await?;
    }
    let (role, admin, _) =
        create_admin_with_password(&pool, "wr-admin", "not-a-real-password").await;
    let settings = test_settings();
    let admin_token = issue_token(&settings, format!("admin:{admin}"), TokenScope::Admin, 900)?;
    let app = build_router(AppState::new(settings).with_mysql(pool.clone()));
    let path = format!("/agent/api/v1/users/{user}/wallet-ledger");
    for caller in [root, child] {
        let (status, page) =
            agent_get_json(app.clone(), &token(caller), format!("{path}?limit=1")).await?;
        assert_eq!(status, StatusCode::OK, "{page}");
        assert_eq!(page["total"], 2);
        assert_eq!(page["ledger"].as_array().unwrap().len(), 1);
        assert_eq!(page["ledger"][0]["user_id"], user);
        assert_eq!(page["ledger"][0]["account_type"], "spot");
        assert_eq!(
            BigDecimal::from_str(page["ledger"][0]["amount"].as_str().unwrap())?,
            BigDecimal::from_str("0.000000000000000001")?
        );
    }
    for caller in [sibling, other] {
        assert_eq!(
            agent_get_json(app.clone(), &token(caller), &path).await?.0,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        agent_get_json(
            app.clone(),
            &token(child),
            format!("/agent/api/v1/users/{outside}/wallet-ledger")
        )
        .await?
        .0,
        StatusCode::NOT_FOUND
    );
    let (_, second) = agent_get_json(
        app.clone(),
        &token(root),
        format!("{path}?limit=1&offset=1"),
    )
    .await?;
    assert_eq!(second["ledger"][0]["change_type"], "deposit");
    for suffix in [
        "change_type=deposit",
        "balance_type=available&ref_type=test&ref_id=ref-1&change_type=deposit",
    ] {
        let (_, page) =
            agent_get_json(app.clone(), &token(child), format!("{path}?{suffix}")).await?;
        assert_eq!(page["total"], 1);
        assert_eq!(page["ledger"][0]["change_type"], "deposit");
    }
    for suffix in [
        "ref_id=absent",
        "asset_id=18446744073709551615",
        "balance_type=locked",
    ] {
        let (_, page) =
            agent_get_json(app.clone(), &token(root), format!("{path}?{suffix}")).await?;
        assert_eq!(page["total"], 0);
        assert_eq!(page["ledger"], json!([]));
    }
    for (account, count) in [("spot", 2), ("margin", 1)] {
        let (status, agent_page) = agent_get_json(
            app.clone(),
            &token(root),
            format!("{path}?account_type={account}"),
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
        let (status, admin_page) = agent_get_json(
            app.clone(),
            &admin_token,
            format!("/admin/api/v1/wallet/ledger?user_id={user}&account_type={account}"),
        )
        .await?;
        assert_eq!(status, StatusCode::OK, "{admin_page}");
        assert_eq!(admin_page["total"], count);
        assert_eq!(admin_page["ledger"][0]["account_type"], account);
        assert_eq!(admin_page["ledger"][0]["id"], agent_page["ledger"][0]["id"]);
    }
    sqlx::query("UPDATE agents SET status='disabled' WHERE id=?")
        .bind(root.agent_id)
        .execute(&pool)
        .await?;
    assert_eq!(
        agent_get_json(app.clone(), &token(child), &path).await?.0,
        StatusCode::UNAUTHORIZED
    );
    sqlx::query("UPDATE agents SET status='active' WHERE id=?")
        .bind(root.agent_id)
        .execute(&pool)
        .await?;
    for (table, expected) in [
        ("wallet_ledger", 2_i64),
        ("margin_wallet_ledger", 1),
        ("wallet_accounts", 0),
        ("margin_wallet_accounts", 0),
    ] {
        let count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {table} WHERE user_id=? AND asset_id=?"
        ))
        .bind(user)
        .bind(asset)
        .fetch_one(&pool)
        .await?;
        assert_eq!(count, expected, "read-only: {table}");
        sqlx::query(&format!(
            "DELETE FROM {table} WHERE user_id=? AND asset_id=?"
        ))
        .bind(user)
        .bind(asset)
        .execute(&pool)
        .await?;
    }
    sqlx::query("DELETE FROM assets WHERE id=?")
        .bind(asset)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM admin_users WHERE id=?")
        .bind(admin)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM admin_roles WHERE id=?")
        .bind(role)
        .execute(&pool)
        .await?;
    cleanup_agent_fixture(&pool, &[child, sibling, root, other], &[user, outside]).await?;
    Ok(())
}
