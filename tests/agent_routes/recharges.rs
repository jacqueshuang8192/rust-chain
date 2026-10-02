use super::*;

// Invalid global asset metadata is tested without racing the options catalogue.
static RECHARGE_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct RechargeFixture {
    _guard: tokio::sync::MutexGuard<'static, ()>,
    pool: MySqlPool,
    app: axum::Router,
    agent: AgentFixture,
    user: u64,
    asset: u64,
    agent_token: String,
    admin_token: String,
    admin: u64,
    role: u64,
}

impl RechargeFixture {
    async fn new() -> Option<Self> {
        let guard = RECHARGE_TEST_LOCK.lock().await;
        let pool = mysql_pool().await?;
        let agent = create_agent(&pool, "recharge").await;
        let user = create_user(&pool, "recharge-target").await;
        refer_user_to_agent(&pool, user, agent.agent_id, 1).await;
        let asset = sqlx::query(
            "INSERT INTO assets (symbol, name, precision_scale, status) VALUES (?, 'Recharge test', 2, 'active')",
        )
        .bind(format!("AR{}", &Uuid::now_v7().simple().to_string()[16..]))
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_id();
        let (role, admin, _) =
            create_admin_with_password(&pool, "recharge", "not-a-real-password").await;
        let settings = test_settings();
        let agent_token = portal_token(agent);
        let admin_token =
            issue_token(&settings, format!("admin:{admin}"), TokenScope::Admin, 900).unwrap();
        let app = build_router(AppState::new(settings).with_mysql(pool.clone()));
        Some(Self {
            _guard: guard,
            pool,
            app,
            agent,
            user,
            asset,
            agent_token,
            admin_token,
            admin,
            role,
        })
    }

    fn body(&self, key: &str, amount: &str) -> Value {
        json!({"asset_id": self.asset, "amount": amount,
            "reason": "customer deposit confirmed", "idempotency_key": key})
    }

    async fn submit(&self, key: &str, amount: &str) -> (StatusCode, Value) {
        request(
            &self.app,
            &self.agent_token,
            "POST",
            &format!("/agent/api/v1/users/{}/recharge", self.user),
            self.body(key, amount),
        )
        .await
    }

    async fn pending(&self, key: &str) -> Value {
        let record = ok(self.submit(key, "1.23").await);
        assert_eq!(record["status"], "pending");
        assert_eq!(record["mode"], "review");
        assert_record(&record, self.agent, self.user, self.asset, "1.23");
        record
    }

    async fn review(&self, id: u64, decision: &str, reason: &str) -> (StatusCode, Value) {
        request(
            &self.app,
            &self.admin_token,
            "POST",
            &format!("/admin/api/v1/agent-recharges/{id}/review"),
            json!({"decision": decision, "reason": reason}),
        )
        .await
    }

    async fn policy(&self, enabled: bool, version: u64) -> (StatusCode, Value) {
        request(
            &self.app,
            &self.admin_token,
            "PATCH",
            &format!(
                "/admin/api/v1/agents/{}/recharge-policy",
                self.agent.agent_id
            ),
            json!({"direct_recharge_enabled": enabled, "expected_version": version,
                "reason": "reviewed agent recharge authorization"}),
        )
        .await
    }

    async fn counts(&self) -> (i64, i64, i64, i64, i64) {
        sqlx::query_as(
            r#"SELECT
                (SELECT COUNT(*) FROM wallet_accounts WHERE asset_id = ?),
                (SELECT COUNT(*) FROM agent_wallet_recharges WHERE asset_id = ?),
                (SELECT COUNT(*) FROM wallet_ledger WHERE asset_id = ?),
                (SELECT COUNT(*) FROM agent_recharge_audits audits
                  JOIN agent_wallet_recharges requests ON requests.id = audits.request_id
                  WHERE requests.asset_id = ?),
                (SELECT COUNT(*) FROM platform_financial_journal WHERE asset_id = ?)"#,
        )
        .bind(self.asset)
        .bind(self.asset)
        .bind(self.asset)
        .bind(self.asset)
        .bind(self.asset)
        .fetch_one(&self.pool)
        .await
        .unwrap()
    }

    async fn balance(&self, user: u64) -> Option<(BigDecimal, BigDecimal, BigDecimal)> {
        sqlx::query_as("SELECT available, frozen, locked FROM wallet_accounts WHERE user_id = ? AND asset_id = ?")
            .bind(user).bind(self.asset).fetch_optional(&self.pool).await.unwrap()
    }

    async fn assert_credit(&self, id: u64, amount: &str) {
        let expected = decimal(amount);
        assert_eq!(
            self.balance(self.user).await.unwrap(),
            (expected.clone(), decimal("0"), decimal("0"))
        );
        assert!(
            self.balance(self.agent.agent_user_id).await.is_none(),
            "recharge must not create or debit the agent wallet"
        );
        let ledgers: Vec<(String, String, String, BigDecimal)> = sqlx::query_as(
            "SELECT change_type, ref_type, ref_id, amount FROM wallet_ledger WHERE user_id = ? AND asset_id = ?",
        ).bind(self.user).bind(self.asset).fetch_all(&self.pool).await.unwrap();
        assert_eq!(
            ledgers,
            vec![(
                "agent_recharge".into(),
                "agent_recharge".into(),
                id.to_string(),
                expected.clone()
            )]
        );
        let legs: Vec<(u64, BigDecimal)> = sqlx::query_as(
            "SELECT asset_id, amount FROM platform_financial_journal WHERE transaction_key = ?",
        )
        .bind(format!("agent_recharge:{id}"))
        .fetch_all(&self.pool)
        .await
        .unwrap();
        assert_eq!(legs.len(), 2);
        assert!(legs.iter().all(|(asset, _)| *asset == self.asset));
        assert_eq!(&legs[0].1 + &legs[1].1, decimal("0"));
        assert!(legs.iter().any(|(_, amount)| *amount == expected));
        assert!(legs.iter().any(|(_, amount)| *amount == -expected.clone()));
        let invalid_audits: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM agent_recharge_audits WHERE request_id = ? AND (actor_id IS NULL OR actor_type = '' OR action = '' OR reason = '' OR created_at IS NULL)",
        ).bind(id).fetch_one(&self.pool).await.unwrap();
        assert_eq!(invalid_audits, 0);
    }

    async fn permissions(&self, permissions: &[&str]) {
        sqlx::query("UPDATE admin_roles SET permissions = ? WHERE id = ?")
            .bind(json!(permissions))
            .bind(self.role)
            .execute(&self.pool)
            .await
            .unwrap();
    }
}

fn portal_token(agent: AgentFixture) -> String {
    issue_token(
        &test_settings(),
        format!("agent:{}", agent.admin_user_id),
        TokenScope::Agent,
        900,
    )
    .unwrap()
}

fn decimal(value: &str) -> BigDecimal {
    BigDecimal::from_str(value).unwrap()
}

async fn request(
    app: &axum::Router,
    token: &str,
    method: &str,
    uri: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
        .await
        .unwrap();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"non_json_body": String::from_utf8_lossy(&bytes)}));
    (status, value)
}

fn ok(result: (StatusCode, Value)) -> Value {
    assert_eq!(result.0, StatusCode::OK, "{}", result.1);
    result.1
}

fn status(result: (StatusCode, Value), expected: StatusCode) {
    assert_eq!(result.0, expected, "{}", result.1);
}

fn assert_record(record: &Value, agent: AgentFixture, user: u64, asset: u64, amount: &str) {
    for field in ["id", "agent_id", "agent_admin_id", "user_id", "asset_id"] {
        let value = record[field]
            .as_u64()
            .unwrap_or_else(|| panic!("{field}: {record}"));
        assert!((1..=9_007_199_254_740_991).contains(&value));
    }
    assert_eq!(record["agent_id"], agent.agent_id);
    assert_eq!(record["agent_admin_id"], agent.admin_user_id);
    assert_eq!(record["user_id"], user);
    assert_eq!(record["asset_id"], asset);
    assert!(record["asset_symbol"].is_string());
    assert_eq!(
        decimal(record["amount"].as_str().expect("money must be a string")),
        decimal(amount)
    );
    assert!(record["created_at"].as_i64().unwrap() > 1_000_000_000_000);
    for field in ["reviewed_by", "review_reason", "reviewed_at", "credited_at"] {
        assert!(record.get(field).is_some(), "missing nullable {field}");
    }
    assert!(record.get("idempotency_key").is_none());
}

#[tokio::test]
async fn agent_recharge_pending_then_concurrent_approval_is_exactly_once() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let record = f.pending("review-once").await;
    let id = record["id"].as_u64().unwrap();
    for field in ["reviewed_by", "review_reason", "reviewed_at", "credited_at"] {
        assert!(record[field].is_null());
    }
    assert_eq!(f.counts().await, (0, 1, 0, 1, 0));
    assert_eq!(ok(f.submit("review-once", "1.2300").await)["id"], id);
    let outcomes =
        futures_util::future::join_all((0..12).map(|_| f.review(id, "approve", "funds confirmed")))
            .await;
    let credited = ok(outcomes[0].clone());
    for outcome in outcomes {
        assert_eq!(ok(outcome), credited);
    }
    assert_eq!(credited["status"], "credited");
    assert_eq!(credited["mode"], "review");
    assert_eq!(credited["reviewed_by"], f.admin);
    assert_eq!(credited["review_reason"], "funds confirmed");
    assert!(credited["reviewed_at"].as_i64().unwrap() >= record["created_at"].as_i64().unwrap());
    assert!(credited["credited_at"].as_i64().unwrap() >= record["created_at"].as_i64().unwrap());
    assert_eq!(f.counts().await, (1, 1, 1, 2, 2));
    f.assert_credit(id, "1.23").await;
    let restarted = build_router(AppState::new(test_settings()).with_mysql(f.pool.clone()));
    assert_eq!(
        ok(request(
            &restarted,
            &f.agent_token,
            "POST",
            &format!("/agent/api/v1/users/{}/recharge", f.user),
            f.body("review-once", "1.23")
        )
        .await)["id"],
        id
    );
    status(
        f.review(id, "approve", "changed reason").await,
        StatusCode::CONFLICT,
    );
    status(
        f.review(id, "reject", "funds confirmed").await,
        StatusCode::CONFLICT,
    );
    let (_, other_admin, _) = create_admin_with_password(&f.pool, "peer", "password").await;
    let other_token = issue_token(
        &test_settings(),
        format!("admin:{other_admin}"),
        TokenScope::Admin,
        900,
    )
    .unwrap();
    status(
        request(
            &f.app,
            &other_token,
            "POST",
            &format!("/admin/api/v1/agent-recharges/{id}/review"),
            json!({"decision":"approve","reason":"funds confirmed"}),
        )
        .await,
        StatusCode::CONFLICT,
    );
    assert_eq!(f.counts().await, (1, 1, 1, 2, 2));
}

#[tokio::test]
async fn agent_recharge_rejection_and_opposite_concurrent_decisions_are_terminal() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let record = f.pending("reject").await;
    let id = record["id"].as_u64().unwrap();
    let rejected = ok(f.review(id, "reject", "unsupported deposit").await);
    assert_eq!(rejected["status"], "rejected");
    assert_eq!(rejected["reviewed_by"], f.admin);
    assert!(rejected["credited_at"].is_null());
    assert_eq!(
        ok(f.review(id, "reject", "unsupported deposit").await),
        rejected
    );
    status(
        f.review(id, "approve", "unsupported deposit").await,
        StatusCode::CONFLICT,
    );
    status(
        f.review(id, "reject", "new reason").await,
        StatusCode::CONFLICT,
    );
    assert_eq!(ok(f.submit("reject", "1.23").await)["id"], id);
    assert_eq!(f.counts().await, (0, 1, 0, 2, 0));

    let race = f.pending("opposite-race").await;
    let race_id = race["id"].as_u64().unwrap();
    let (approved, denied) = tokio::join!(
        f.review(race_id, "approve", "race decision"),
        f.review(race_id, "reject", "race decision")
    );
    assert!(
        (approved.0 == StatusCode::OK && denied.0 == StatusCode::CONFLICT)
            || (approved.0 == StatusCode::CONFLICT && denied.0 == StatusCode::OK),
        "approve={approved:?}, reject={denied:?}"
    );
    if approved.0 == StatusCode::OK {
        f.assert_credit(race_id, "1.23").await;
        assert_eq!(f.counts().await, (1, 2, 1, 4, 2));
    } else {
        assert_eq!(f.counts().await, (0, 2, 0, 4, 0));
    }
}

#[tokio::test]
async fn agent_recharge_direct_opt_in_revocation_does_not_inherit_or_recredit() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let options = ok(request(
        &f.app,
        &f.agent_token,
        "GET",
        "/agent/api/v1/recharges/options",
        Value::Null,
    )
    .await);
    assert_eq!(options["direct_recharge_enabled"], false);
    let asset = options["assets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["asset_id"] == f.asset)
        .unwrap();
    assert_eq!(asset["precision_scale"], 2);
    let pending = f.pending("before-opt-in").await;
    let policy = ok(f.policy(true, 0).await);
    assert_eq!(policy["version"], 1);
    assert_eq!(policy["direct_recharge_enabled"], true);
    assert_eq!(
        f.counts().await,
        (0, 1, 0, 1, 0),
        "authorizing must not credit old requests"
    );
    status(f.policy(false, 0).await, StatusCode::CONFLICT);

    let outcomes =
        futures_util::future::join_all((0..12).map(|_| f.submit("direct-once", "1.23"))).await;
    let direct = ok(outcomes[0].clone());
    for outcome in outcomes {
        assert_eq!(ok(outcome), direct);
    }
    assert_eq!(direct["mode"], "direct");
    assert_eq!(direct["status"], "credited");
    f.assert_credit(direct["id"].as_u64().unwrap(), "1.23")
        .await;
    let after_direct = f.counts().await;
    assert!(after_direct.3 >= 2, "direct credit needs a required audit");

    let child = create_child_agent(&f.pool, f.agent, "recharge-child").await;
    let child_user = create_user(&f.pool, "child-recharge").await;
    refer_user_to_agent(&f.pool, child_user, child.agent_id, 1).await;
    let child_options = ok(request(
        &f.app,
        &portal_token(child),
        "GET",
        "/agent/api/v1/recharges/options",
        Value::Null,
    )
    .await);
    assert_eq!(child_options["direct_recharge_enabled"], false);
    let child_record = ok(request(
        &f.app,
        &portal_token(child),
        "POST",
        &format!("/agent/api/v1/users/{child_user}/recharge"),
        f.body("child-review", "1.23"),
    )
    .await);
    assert_eq!(child_record["mode"], "review");
    assert_eq!(child_record["status"], "pending");
    assert!(f.balance(child_user).await.is_none());

    assert_eq!(ok(f.policy(false, 1).await)["version"], 2);
    let replay = ok(f.submit("direct-once", "1.23").await);
    assert_eq!(replay["id"], direct["id"]);
    assert_eq!(replay["mode"], "direct");
    assert_eq!(replay["status"], "credited");
    let next = f.pending("after-revoke").await;
    assert_ne!(next["id"], direct["id"]);
    assert_eq!(
        ok(f.submit("before-opt-in", "1.23").await)["id"],
        pending["id"]
    );
    f.assert_credit(direct["id"].as_u64().unwrap(), "1.23")
        .await;
    let counts = f.counts().await;
    assert_eq!((counts.0, counts.1, counts.2, counts.4), (1, 4, 1, 2));
}

#[tokio::test]
async fn agent_recharge_scope_history_and_reassignment_use_current_subtree() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let child = create_child_agent(&f.pool, f.agent, "scope-child").await;
    let sibling = create_child_agent(&f.pool, f.agent, "scope-sibling").await;
    let grandchild = create_child_agent(&f.pool, child, "scope-grandchild").await;
    let other = create_agent(&f.pool, "scope-other").await;
    let mut users = Vec::new();
    for owner in [child, sibling, grandchild, other] {
        let user = create_user(&f.pool, "scope-user").await;
        refer_user_to_agent(&f.pool, user, owner.agent_id, 1).await;
        users.push(user);
    }
    let unassigned = create_user(&f.pool, "scope-unassigned").await;
    create_unassigned_referral(&f.pool, unassigned).await;
    refer_user_to_agent(&f.pool, child.agent_user_id, child.agent_id, 1).await;
    for (index, target) in [
        f.user,
        users[1],
        users[3],
        unassigned,
        9_007_199_254_740_990,
    ]
    .into_iter()
    .enumerate()
    {
        status(
            request(
                &f.app,
                &portal_token(child),
                "POST",
                &format!(
                    "/agent/api/v1/users/{target}/recharge?agent_id={}",
                    f.agent.agent_id
                ),
                f.body(&format!("outside-{index}"), "1.23"),
            )
            .await,
            StatusCode::NOT_FOUND,
        );
    }
    let self_result = request(
        &f.app,
        &portal_token(child),
        "POST",
        &format!("/agent/api/v1/users/{}/recharge", child.agent_user_id),
        f.body("self", "1.23"),
    )
    .await;
    assert!(
        matches!(
            self_result.0,
            StatusCode::BAD_REQUEST | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
        ),
        "{self_result:?}"
    );
    assert_eq!(f.counts().await, (0, 0, 0, 0, 0));
    let root_request = f.pending("root-own").await;
    let child_request = ok(request(
        &f.app,
        &portal_token(child),
        "POST",
        &format!("/agent/api/v1/users/{}/recharge", users[0]),
        f.body("child-own", "1.23"),
    )
    .await);
    let descendant_request = ok(request(
        &f.app,
        &portal_token(child),
        "POST",
        &format!("/agent/api/v1/users/{}/recharge", users[2]),
        f.body("child-descendant", "1.23"),
    )
    .await);
    let root_descendant = ok(request(
        &f.app,
        &f.agent_token,
        "POST",
        &format!("/agent/api/v1/users/{}/recharge", users[0]),
        f.body("root-descendant", "1.23"),
    )
    .await);
    let history = ok(request(
        &f.app,
        &f.agent_token,
        "GET",
        "/agent/api/v1/recharges?status=pending&limit=1&offset=1",
        Value::Null,
    )
    .await);
    assert_eq!(history["total"], 2);
    assert_eq!(history["recharges"].as_array().unwrap().len(), 1);
    assert_eq!(
        history["recharges"][0]["agent_admin_id"],
        f.agent.admin_user_id
    );
    assert_eq!(history["recharges"][0]["id"], root_request["id"]);
    let child_history = ok(request(
        &f.app,
        &portal_token(child),
        "GET",
        "/agent/api/v1/recharges?status=pending&limit=1&offset=0",
        Value::Null,
    )
    .await);
    assert_eq!(child_history["total"], 2);
    assert_eq!(
        child_history["recharges"][0]["id"],
        descendant_request["id"]
    );
    assert_ne!(child_history["recharges"][0]["id"], root_descendant["id"]);

    sqlx::query("UPDATE user_referrals SET root_agent_id = ? WHERE user_id = ?")
        .bind(other.agent_id)
        .bind(users[0])
        .execute(&f.pool)
        .await
        .unwrap();
    let stale_id = child_request["id"].as_u64().unwrap();
    let approve = f.review(stale_id, "approve", "stale owner").await;
    assert!(
        matches!(approve.0, StatusCode::NOT_FOUND | StatusCode::CONFLICT),
        "{approve:?}"
    );
    assert!(f.balance(users[0]).await.is_none());
    assert_eq!(
        ok(f.review(stale_id, "reject", "ownership changed").await)["status"],
        "rejected"
    );
    for token in [&f.agent_token, &portal_token(child)] {
        let history = ok(request(
            &f.app,
            token,
            "GET",
            &format!("/agent/api/v1/recharges?user_id={}", users[0]),
            Value::Null,
        )
        .await);
        assert_eq!(history["total"], 0);
        assert!(history["recharges"].as_array().unwrap().is_empty());
    }
    let queue = ok(request(
        &f.app,
        &f.admin_token,
        "GET",
        &format!(
            "/admin/api/v1/agent-recharges?agent_id={}&user_id={}&status=rejected&limit=1&offset=0",
            child.agent_id, users[0]
        ),
        Value::Null,
    )
    .await);
    assert_eq!(queue["total"], 1);
    assert_eq!(queue["recharges"][0]["id"], stale_id);
}

#[tokio::test]
async fn agent_recharge_idempotency_binds_account_user_asset_amount_reason_and_raw_key() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let original = f.pending("CaseSensitiveKey").await;
    for (field, replacement) in [
        ("amount", json!("2.23")),
        ("asset_id", json!(f.asset + 1)),
        ("reason", json!("different intent")),
    ] {
        let mut body = f.body("CaseSensitiveKey", "1.23");
        body[field] = replacement;
        status(
            request(
                &f.app,
                &f.agent_token,
                "POST",
                &format!("/agent/api/v1/users/{}/recharge", f.user),
                body,
            )
            .await,
            StatusCode::CONFLICT,
        );
    }
    let other_user = create_user(&f.pool, "same-key-target").await;
    refer_user_to_agent(&f.pool, other_user, f.agent.agent_id, 1).await;
    status(
        request(
            &f.app,
            &f.agent_token,
            "POST",
            &format!("/agent/api/v1/users/{other_user}/recharge"),
            f.body("CaseSensitiveKey", "1.23"),
        )
        .await,
        StatusCode::CONFLICT,
    );
    let lower = ok(f.submit("casesensitivekey", "1.23").await);
    assert_ne!(
        lower["id"], original["id"],
        "VARBINARY keys must remain case sensitive"
    );
    let second_admin = sqlx::query(
        "INSERT INTO agent_admin_users (agent_id, username, password_hash) VALUES (?, ?, 'test-hash')",
    ).bind(f.agent.agent_id).bind(format!("recharge-second-{}", Uuid::now_v7().simple()))
        .execute(&f.pool).await.unwrap().last_insert_id();
    let second = AgentFixture {
        admin_user_id: second_admin,
        ..f.agent
    };
    let separate = ok(request(
        &f.app,
        &portal_token(second),
        "POST",
        &format!("/agent/api/v1/users/{}/recharge", f.user),
        f.body("CaseSensitiveKey", "1.23"),
    )
    .await);
    assert_ne!(separate["id"], original["id"]);
    let history = ok(request(
        &f.app,
        &portal_token(second),
        "GET",
        "/agent/api/v1/recharges",
        Value::Null,
    )
    .await);
    assert_eq!(history["total"], 1);
    assert_eq!(history["recharges"][0]["id"], separate["id"]);
    assert_eq!(f.counts().await, (0, 3, 0, 3, 0));
}

#[tokio::test]
async fn agent_recharge_rbac_requires_exact_independent_permissions() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let id = f.pending("permission-review").await["id"].as_u64().unwrap();
    let queue = format!(
        "/admin/api/v1/agent-recharges?agent_id={}",
        f.agent.agent_id
    );
    let policy = format!("/admin/api/v1/agents/{}/recharge-policy", f.agent.agent_id);
    let detail = format!("/admin/api/v1/agent-recharges/{id}");
    for permissions in [&["agents.write"][..], &["agents.read"][..], &[][..]] {
        f.permissions(permissions).await;
        status(
            request(&f.app, &f.admin_token, "GET", &queue, Value::Null).await,
            StatusCode::FORBIDDEN,
        );
        status(
            request(&f.app, &f.admin_token, "GET", &detail, Value::Null).await,
            StatusCode::FORBIDDEN,
        );
        status(
            f.review(id, "approve", "permission check").await,
            StatusCode::FORBIDDEN,
        );
        status(f.policy(true, 0).await, StatusCode::FORBIDDEN);
        let read = request(&f.app, &f.admin_token, "GET", &policy, Value::Null).await;
        status(
            read,
            if permissions.contains(&"agents.read") {
                StatusCode::OK
            } else {
                StatusCode::FORBIDDEN
            },
        );
    }
    f.permissions(&["agents.recharges.read"]).await;
    assert_eq!(
        ok(request(&f.app, &f.admin_token, "GET", &detail, Value::Null).await)["id"],
        id
    );
    assert_eq!(
        ok(request(&f.app, &f.admin_token, "GET", &queue, Value::Null).await)["total"],
        1
    );
    status(
        f.review(id, "approve", "permission check").await,
        StatusCode::FORBIDDEN,
    );
    status(f.policy(true, 0).await, StatusCode::FORBIDDEN);
    f.permissions(&["agents.recharges.authorize"]).await;
    status(
        request(&f.app, &f.admin_token, "GET", &detail, Value::Null).await,
        StatusCode::FORBIDDEN,
    );
    assert_eq!(ok(f.policy(true, 0).await)["version"], 1);
    status(
        request(&f.app, &f.admin_token, "GET", &queue, Value::Null).await,
        StatusCode::FORBIDDEN,
    );
    status(
        f.review(id, "approve", "permission check").await,
        StatusCode::FORBIDDEN,
    );
    f.permissions(&["agents.recharges.review"]).await;
    status(
        request(&f.app, &f.admin_token, "GET", &detail, Value::Null).await,
        StatusCode::FORBIDDEN,
    );
    status(f.policy(false, 1).await, StatusCode::FORBIDDEN);
    assert_eq!(
        ok(f.review(id, "approve", "permission check").await)["status"],
        "credited"
    );
    f.assert_credit(id, "1.23").await;
    status(
        request(&f.app, &f.agent_token, "GET", &queue, Value::Null).await,
        StatusCode::FORBIDDEN,
    );
    status(
        request(
            &f.app,
            &f.admin_token,
            "POST",
            &format!("/agent/api/v1/users/{}/recharge", f.user),
            f.body("wrong-scope", "1.23"),
        )
        .await,
        StatusCode::FORBIDDEN,
    );
    let unauthenticated = f
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/agent/api/v1/recharges/options")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn agent_recharge_detail_reconciles_another_admin_terminal_result_without_mutation() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let pending = f.pending("detail-reconcile").await;
    let id = pending["id"].as_u64().unwrap();
    let uri = format!("/admin/api/v1/agent-recharges/{id}");
    assert_eq!(
        ok(request(&f.app, &f.admin_token, "GET", &uri, Value::Null).await),
        pending
    );
    let (_, other_admin, _) = create_admin_with_password(&f.pool, "detail", "password").await;
    let other_token = issue_token(
        &test_settings(),
        format!("admin:{other_admin}"),
        TokenScope::Admin,
        900,
    )
    .unwrap();
    let approved = ok(request(
        &f.app,
        &other_token,
        "POST",
        &format!("{uri}/review"),
        json!({"decision":"approve","reason":"other operator confirmation"}),
    )
    .await);
    status(
        f.review(id, "approve", "local stale confirmation").await,
        StatusCode::CONFLICT,
    );
    f.permissions(&["agents.recharges.read"]).await;
    let baseline = f.counts().await;
    let balance = f.balance(f.user).await;
    let other_agent = create_agent(&f.pool, "detail-new").await;
    sqlx::query("UPDATE user_referrals SET root_agent_id = ? WHERE user_id = ?")
        .bind(other_agent.agent_id)
        .bind(f.user)
        .execute(&f.pool)
        .await
        .unwrap();
    let detail = ok(request(&f.app, &f.admin_token, "GET", &uri, Value::Null).await);
    assert_eq!(detail, approved);
    assert_eq!(detail["reviewed_by"], other_admin);
    assert_record(&detail, f.agent, f.user, f.asset, "1.23");
    status(
        request(
            &f.app,
            &f.admin_token,
            "GET",
            "/admin/api/v1/agent-recharges/9007199254740990",
            Value::Null,
        )
        .await,
        StatusCode::NOT_FOUND,
    );
    status(
        request(&f.app, &f.agent_token, "GET", &uri, Value::Null).await,
        StatusCode::FORBIDDEN,
    );
    let unauthenticated = f
        .app
        .clone()
        .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(f.counts().await, baseline);
    assert_eq!(f.balance(f.user).await, balance);
}

#[tokio::test]
async fn agent_recharge_scope_locks_do_not_block_unrelated_agent_authorization() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let unrelated = create_agent(&f.pool, "unrelated").await;
    let mut asset_lock = f.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM assets WHERE id = ? FOR UPDATE")
        .bind(f.asset)
        .fetch_one(&mut *asset_lock)
        .await
        .unwrap();
    let app = f.app.clone();
    let token = f.agent_token.clone();
    let uri = format!("/agent/api/v1/users/{}/recharge", f.user);
    let body = f.body("narrow-scope-lock", "1.23");
    let submission = tokio::spawn(async move { request(&app, &token, "POST", &uri, body).await });

    // Observe the real request after scope locking, before allowing asset validation.
    let waiting = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM performance_schema.data_lock_waits waits \
                 JOIN performance_schema.data_locks locks \
                 ON locks.ENGINE_LOCK_ID = waits.REQUESTING_ENGINE_LOCK_ID \
                 WHERE locks.OBJECT_SCHEMA = DATABASE() AND locks.OBJECT_NAME = 'assets' \
                 AND locks.INDEX_NAME = 'PRIMARY' AND locks.LOCK_DATA = ?",
            )
            .bind(f.asset.to_string())
            .fetch_one(&f.pool)
            .await
            .unwrap();
            if count > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    let policy = tokio::time::timeout(std::time::Duration::from_secs(2),
        request(&f.app, &f.admin_token, "PATCH",
            &format!("/admin/api/v1/agents/{}/recharge-policy", unrelated.agent_id),
            json!({"direct_recharge_enabled":true,"expected_version":0,"reason":"independent scope"})),
    ).await;
    asset_lock.rollback().await.unwrap();
    let record = ok(submission.await.unwrap());
    assert!(
        waiting.is_ok(),
        "did not observe the recharge waiting on its asset row"
    );
    let policy = ok(policy.expect("recharge locked an unrelated agent during asset wait"));
    assert_eq!(policy["direct_recharge_enabled"], true);
    assert_eq!(policy["version"], 1);
    assert_eq!(record["status"], "pending");
    assert_eq!(f.counts().await, (0, 1, 0, 1, 0));
}

#[tokio::test]
async fn agent_recharge_rejects_missing_duplicate_and_noncanonical_ancestor_paths() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let child = create_child_agent(&f.pool, f.agent, "path").await;
    sqlx::query("UPDATE user_referrals SET root_agent_id = ? WHERE user_id = ?")
        .bind(child.agent_id)
        .bind(f.user)
        .execute(&f.pool)
        .await
        .unwrap();
    let root = f.agent.agent_id;
    let child_id = child.agent_id;
    for (index, path) in [
        format!("/agent:{root}/agent:9007199254740990/agent:{child_id}"),
        format!("/agent:{root}/agent:{root}/agent:{child_id}"),
        format!("/agent:{root}/agent:0{child_id}"),
        format!("/agent:{root}/agent:0/agent:{child_id}"),
        format!("/agent:{root}/agent:9007199254740989/agent:9007199254740990/agent:{child_id}"),
    ]
    .into_iter()
    .enumerate()
    {
        sqlx::query("UPDATE agents SET path = ? WHERE id = ?")
            .bind(path)
            .bind(child_id)
            .execute(&f.pool)
            .await
            .unwrap();
        status(
            f.submit(&format!("bad-path-{index}"), "1.23").await,
            StatusCode::NOT_FOUND,
        );
        assert_eq!(f.counts().await, (0, 0, 0, 0, 0));
    }
    sqlx::query("UPDATE agents SET path = ? WHERE id = ?")
        .bind(format!("/agent:{root}/agent:{child_id}"))
        .bind(child_id)
        .execute(&f.pool)
        .await
        .unwrap();
    assert_eq!(
        ok(f.submit("valid-path", "1.23").await)["status"],
        "pending"
    );
}

#[tokio::test]
async fn agent_recharge_active_chain_and_portal_account_are_revalidated_at_approval() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let child = create_child_agent(&f.pool, f.agent, "active-chain").await;
    let user = create_user(&f.pool, "active-chain-target").await;
    refer_user_to_agent(&f.pool, user, child.agent_id, 1).await;
    let token = portal_token(child);
    for (table, target, label) in [
        ("agents", f.agent.agent_id, "ancestor"),
        ("agents", child.agent_id, "agent"),
        ("agent_admin_users", child.admin_user_id, "portal"),
    ] {
        let record = ok(request(
            &f.app,
            &token,
            "POST",
            &format!("/agent/api/v1/users/{user}/recharge"),
            f.body(label, "1.23"),
        )
        .await);
        sqlx::query(&format!(
            "UPDATE {table} SET status = 'disabled' WHERE id = ?"
        ))
        .bind(target)
        .execute(&f.pool)
        .await
        .unwrap();
        status(
            request(
                &f.app,
                &token,
                "POST",
                &format!("/agent/api/v1/users/{user}/recharge"),
                f.body(&format!("{label}-new"), "1.23"),
            )
            .await,
            StatusCode::UNAUTHORIZED,
        );
        let rejected = f
            .review(record["id"].as_u64().unwrap(), "approve", "inactive chain")
            .await;
        assert!(
            matches!(
                rejected.0,
                StatusCode::UNAUTHORIZED | StatusCode::CONFLICT | StatusCode::NOT_FOUND
            ),
            "{label}: {rejected:?}"
        );
        assert!(f.balance(user).await.is_none());
        let stored: String =
            sqlx::query_scalar("SELECT status FROM agent_wallet_recharges WHERE id = ?")
                .bind(record["id"].as_u64().unwrap())
                .fetch_one(&f.pool)
                .await
                .unwrap();
        assert_eq!(stored, "pending");
        sqlx::query(&format!(
            "UPDATE {table} SET status = 'active' WHERE id = ?"
        ))
        .bind(target)
        .execute(&f.pool)
        .await
        .unwrap();
    }
    assert_eq!(f.counts().await, (0, 3, 0, 3, 0));
}

#[tokio::test]
async fn agent_recharge_precision_validation_and_storage_overflow_never_partially_write() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    for (precision, amount) in [(0, "1.1"), (2, "0.001"), (2, "0"), (2, "-1")] {
        sqlx::query("UPDATE assets SET precision_scale = ? WHERE id = ?")
            .bind(precision)
            .bind(f.asset)
            .execute(&f.pool)
            .await
            .unwrap();
        status(
            f.submit(&format!("invalid-{precision}-{amount}"), amount)
                .await,
            StatusCode::BAD_REQUEST,
        );
        assert_eq!(f.counts().await, (0, 0, 0, 0, 0));
    }
    for amount in ["0.0000000000000000001", "100000000000000000000", "invalid"] {
        status(
            f.submit(&format!("invalid-wire-{amount}"), amount).await,
            StatusCode::UNPROCESSABLE_ENTITY,
        );
        assert_eq!(f.counts().await, (0, 0, 0, 0, 0));
    }
    for precision in [-1, 19] {
        sqlx::query("UPDATE assets SET precision_scale = ? WHERE id = ?")
            .bind(precision)
            .bind(f.asset)
            .execute(&f.pool)
            .await
            .unwrap();
        status(
            f.submit(&format!("bad-config-{precision}"), "1").await,
            StatusCode::INTERNAL_SERVER_ERROR,
        );
        assert_eq!(f.counts().await, (0, 0, 0, 0, 0));
    }
    sqlx::query("UPDATE assets SET precision_scale = 2 WHERE id = ?")
        .bind(f.asset)
        .execute(&f.pool)
        .await
        .unwrap();
    let pending = f.pending("wallet-overflow").await;
    let id = pending["id"].as_u64().unwrap();
    let maximum = "99999999999999999999.999999999999999999";
    sqlx::query("INSERT INTO wallet_accounts (user_id, asset_id, available) VALUES (?, ?, ?)")
        .bind(f.user)
        .bind(f.asset)
        .bind(decimal(maximum))
        .execute(&f.pool)
        .await
        .unwrap();
    let baseline = f.counts().await;
    let overflow = f.review(id, "approve", "overflow check").await;
    assert!(
        matches!(
            overflow.0,
            StatusCode::BAD_REQUEST | StatusCode::CONFLICT | StatusCode::INTERNAL_SERVER_ERROR
        ),
        "{overflow:?}"
    );
    assert_eq!(f.counts().await, baseline);
    assert_eq!(f.balance(f.user).await.unwrap().0, decimal(maximum));
    let pending_status: String =
        sqlx::query_scalar("SELECT status FROM agent_wallet_recharges WHERE id = ?")
            .bind(id)
            .fetch_one(&f.pool)
            .await
            .unwrap();
    assert_eq!(pending_status, "pending");
    ok(f.policy(true, 0).await);
    let overflow = f.submit("direct-overflow", "1.23").await;
    assert!(
        matches!(
            overflow.0,
            StatusCode::BAD_REQUEST | StatusCode::CONFLICT | StatusCode::INTERNAL_SERVER_ERROR
        ),
        "{overflow:?}"
    );
    assert_eq!(f.counts().await, baseline);
}

#[tokio::test]
async fn agent_recharge_audit_failure_rolls_back_pending_review_and_direct_credit() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let trigger = format!("recharge_audit_{}", Uuid::now_v7().simple());
    sqlx::raw_sql(&format!(
        "CREATE TRIGGER {trigger} BEFORE INSERT ON agent_recharge_audits FOR EACH ROW BEGIN \
        IF (NEW.actor_type = 'agent' AND NEW.actor_id = {}) OR (NEW.actor_type = 'admin' AND NEW.actor_id = {}) \
        THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'target recharge audit failure'; END IF; END",
        f.agent.admin_user_id, f.admin,
    )).execute(&f.pool).await.unwrap();
    let failed = f.submit("audit-pending", "1.23").await;
    sqlx::raw_sql(&format!("DROP TRIGGER {trigger}"))
        .execute(&f.pool)
        .await
        .unwrap();
    status(failed, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(f.counts().await, (0, 0, 0, 0, 0));
    let record = f.pending("audit-pending").await;
    let id = record["id"].as_u64().unwrap();

    sqlx::raw_sql(&format!(
        "CREATE TRIGGER {trigger} BEFORE INSERT ON agent_recharge_audits FOR EACH ROW BEGIN \
        IF NEW.request_id = {id} THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'review audit failure'; END IF; END",
    )).execute(&f.pool).await.unwrap();
    let failed = f.review(id, "approve", "audit retry").await;
    let reject_failed = f.review(id, "reject", "audit retry").await;
    sqlx::raw_sql(&format!("DROP TRIGGER {trigger}"))
        .execute(&f.pool)
        .await
        .unwrap();
    status(failed, StatusCode::INTERNAL_SERVER_ERROR);
    status(reject_failed, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(f.counts().await, (0, 1, 0, 1, 0));
    assert_eq!(
        ok(f.review(id, "approve", "audit retry").await)["status"],
        "credited"
    );
    f.assert_credit(id, "1.23").await;

    ok(f.policy(true, 0).await);
    let baseline = f.counts().await;
    let balance = f.balance(f.user).await;
    sqlx::raw_sql(&format!(
        "CREATE TRIGGER {trigger} BEFORE INSERT ON agent_recharge_audits FOR EACH ROW BEGIN \
        IF NEW.actor_id = {} AND NEW.actor_type = 'agent' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'direct audit failure'; END IF; END",
        f.agent.admin_user_id,
    )).execute(&f.pool).await.unwrap();
    let failed = f.submit("audit-direct", "1.23").await;
    sqlx::raw_sql(&format!("DROP TRIGGER {trigger}"))
        .execute(&f.pool)
        .await
        .unwrap();
    status(failed, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(f.counts().await, baseline);
    assert_eq!(f.balance(f.user).await, balance);
    assert_eq!(
        ok(f.submit("audit-direct", "1.23").await)["status"],
        "credited"
    );
    assert_eq!(f.balance(f.user).await.unwrap().0, decimal("2.46"));
}

#[tokio::test]
async fn agent_recharge_journal_failure_rolls_back_all_effects_and_retry_succeeds() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let record = f.pending("journal-rollback").await;
    let id = record["id"].as_u64().unwrap();
    let trigger = format!("recharge_journal_{}", Uuid::now_v7().simple());
    sqlx::raw_sql(&format!(
        "CREATE TRIGGER {trigger} BEFORE INSERT ON platform_financial_journal FOR EACH ROW BEGIN \
        IF NEW.transaction_key = 'agent_recharge:{id}' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'target journal failure'; END IF; END",
    )).execute(&f.pool).await.unwrap();
    let failed = f.review(id, "approve", "journal retry").await;
    sqlx::raw_sql(&format!("DROP TRIGGER {trigger}"))
        .execute(&f.pool)
        .await
        .unwrap();
    status(failed, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(f.counts().await, (0, 1, 0, 1, 0));
    assert_eq!(
        ok(f.review(id, "approve", "journal retry").await)["status"],
        "credited"
    );
    f.assert_credit(id, "1.23").await;
}

#[tokio::test]
async fn agent_recharge_distinct_users_accounts_and_cross_target_same_key_do_not_deadlock() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let other = create_agent(&f.pool, "parallel").await;
    let mut commands = Vec::new();
    for index in 0..12 {
        let owner = if index < 6 { f.agent } else { other };
        let user = create_user(&f.pool, "parallel-target").await;
        refer_user_to_agent(&f.pool, user, owner.agent_id, 1).await;
        commands.push((
            portal_token(owner),
            format!("/agent/api/v1/users/{user}/recharge"),
            f.body(&format!("distinct-{index}"), "1.23"),
        ));
    }
    let outcomes = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        futures_util::future::join_all(
            commands
                .iter()
                .map(|(token, uri, body)| request(&f.app, token, "POST", uri, body.clone())),
        ),
    )
    .await
    .expect("distinct recharge requests deadlocked");
    let mut ids = std::collections::BTreeSet::new();
    for outcome in outcomes {
        let record = ok(outcome);
        assert_eq!(record["status"], "pending");
        assert!(ids.insert(record["id"].as_u64().unwrap()));
    }
    assert_eq!(f.counts().await, (0, 12, 0, 12, 0));
    let other_user = create_user(&f.pool, "cross-key-user").await;
    refer_user_to_agent(&f.pool, other_user, f.agent.agent_id, 1).await;
    let first_uri = format!("/agent/api/v1/users/{}/recharge", f.user);
    let second_uri = format!("/agent/api/v1/users/{other_user}/recharge");
    let (first, second) = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        tokio::join!(
            request(
                &f.app,
                &f.agent_token,
                "POST",
                &first_uri,
                f.body("cross-target-key", "1.23")
            ),
            request(
                &f.app,
                &f.agent_token,
                "POST",
                &second_uri,
                f.body("cross-target-key", "1.23")
            ),
        )
    })
    .await
    .expect("same-key cross-target requests deadlocked");
    assert!(
        (first.0 == StatusCode::OK && second.0 == StatusCode::CONFLICT)
            || (first.0 == StatusCode::CONFLICT && second.0 == StatusCode::OK),
        "first={first:?}, second={second:?}",
    );
    let winner = if first.0 == StatusCode::OK {
        first.1
    } else {
        second.1
    };
    let stored: (u64, u64) = sqlx::query_as(
        "SELECT id, user_id FROM agent_wallet_recharges WHERE agent_admin_id = ? AND idempotency_key = ?",
    ).bind(f.agent.admin_user_id).bind(b"cross-target-key".as_slice())
        .fetch_one(&f.pool).await.unwrap();
    assert_eq!(
        stored,
        (
            winner["id"].as_u64().unwrap(),
            winner["user_id"].as_u64().unwrap()
        )
    );
    assert_eq!(f.counts().await, (0, 13, 0, 13, 0));
}

#[tokio::test]
async fn agent_recharge_exact_precision_trailing_zeros_and_config_change_replay() {
    for (precision, amount, expected) in [
        (0, "123.0000", "123"),
        (2, "1.2300", "1.23"),
        (18, "0.000000000000000001", "0.000000000000000001"),
    ] {
        let Some(f) = RechargeFixture::new().await else {
            return;
        };
        sqlx::query("UPDATE assets SET precision_scale = ? WHERE id = ?")
            .bind(precision)
            .bind(f.asset)
            .execute(&f.pool)
            .await
            .unwrap();
        ok(f.policy(true, 0).await);
        let record = ok(f.submit("precision-success", amount).await);
        assert_record(&record, f.agent, f.user, f.asset, expected);
        let id = record["id"].as_u64().unwrap();
        f.assert_credit(id, expected).await;
        sqlx::query("UPDATE assets SET status = 'disabled', precision_scale = 0 WHERE id = ?")
            .bind(f.asset)
            .execute(&f.pool)
            .await
            .unwrap();
        assert_eq!(ok(f.submit("precision-success", amount).await)["id"], id);
        status(
            f.submit("disabled-new", amount).await,
            StatusCode::BAD_REQUEST,
        );
        assert_eq!(f.counts().await, (1, 1, 1, 1, 2));
        let options = ok(request(
            &f.app,
            &f.agent_token,
            "GET",
            "/agent/api/v1/recharges/options",
            Value::Null,
        )
        .await);
        assert!(
            !options["assets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|asset| asset["asset_id"] == f.asset)
        );
    }
}

#[tokio::test]
async fn agent_recharge_invalid_wire_fields_filters_and_inactive_target_are_atomic() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let uri = format!("/agent/api/v1/users/{}/recharge", f.user);
    for amount in [json!(1.23), Value::Null, json!(true)] {
        let mut body = f.body("wire", "1.23");
        body["amount"] = amount;
        status(
            request(&f.app, &f.agent_token, "POST", &uri, body).await,
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }
    for field in ["asset_id", "amount", "reason", "idempotency_key"] {
        let mut body = f.body("missing", "1.23");
        body.as_object_mut().unwrap().remove(field);
        status(
            request(&f.app, &f.agent_token, "POST", &uri, body).await,
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }
    for (field, value) in [
        ("reason", json!("   ")),
        ("reason", json!("x".repeat(501))),
        ("idempotency_key", json!("")),
        ("idempotency_key", json!("x".repeat(129))),
    ] {
        let mut body = f.body("invalid-field", "1.23");
        body[field] = value;
        status(
            request(&f.app, &f.agent_token, "POST", &uri, body).await,
            StatusCode::BAD_REQUEST,
        );
    }
    for field in ["agent_id", "direct_recharge_enabled"] {
        let mut body = f.body("injected-identity", "1.23");
        body[field] = json!(1);
        status(
            request(&f.app, &f.agent_token, "POST", &uri, body).await,
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }
    for (token, uri) in [
        (&f.agent_token, "/agent/api/v1/recharges?status=approved"),
        (
            &f.admin_token,
            "/admin/api/v1/agent-recharges?status=approved",
        ),
    ] {
        status(
            request(&f.app, token, "GET", uri, Value::Null).await,
            StatusCode::BAD_REQUEST,
        );
    }
    assert_eq!(f.counts().await, (0, 0, 0, 0, 0));
    let record = f.pending("target-disabled").await;
    let id = record["id"].as_u64().unwrap();
    sqlx::query("UPDATE users SET status = 'disabled' WHERE id = ?")
        .bind(f.user)
        .execute(&f.pool)
        .await
        .unwrap();
    status(
        f.submit("target-disabled-new", "1.23").await,
        StatusCode::CONFLICT,
    );
    status(
        f.review(id, "approve", "disabled user").await,
        StatusCode::CONFLICT,
    );
    assert_eq!(f.counts().await, (0, 1, 0, 1, 0));
    assert_eq!(
        ok(f.review(id, "reject", "disabled user").await)["status"],
        "rejected"
    );
    assert_eq!(f.counts().await, (0, 1, 0, 2, 0));
}

#[tokio::test]
async fn agent_recharge_policy_audit_failure_and_concurrent_versions_preserve_authorization() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let uri = format!("/admin/api/v1/agents/{}/recharge-policy", f.agent.agent_id);
    let before = ok(request(&f.app, &f.admin_token, "GET", &uri, Value::Null).await);
    assert_eq!(before["version"], 0);
    assert_eq!(before["direct_recharge_enabled"], false);
    status(
        request(
            &f.app,
            &f.admin_token,
            "PATCH",
            &uri,
            json!({"direct_recharge_enabled":true, "expected_version":0, "reason":"  "}),
        )
        .await,
        StatusCode::BAD_REQUEST,
    );
    let trigger = format!("recharge_policy_{}", Uuid::now_v7().simple());
    sqlx::raw_sql(&format!(
        "CREATE TRIGGER {trigger} BEFORE INSERT ON admin_audit_logs FOR EACH ROW BEGIN \
        IF NEW.admin_id = {} AND NEW.target_type = 'agent' AND NEW.target_id = '{}' \
        AND NEW.action = 'agent_recharge.authorize' THEN SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'target policy audit failure'; END IF; END",
        f.admin, f.agent.agent_id,
    )).execute(&f.pool).await.unwrap();
    let failed = f.policy(true, 0).await;
    sqlx::raw_sql(&format!("DROP TRIGGER {trigger}"))
        .execute(&f.pool)
        .await
        .unwrap();
    status(failed, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        ok(request(&f.app, &f.admin_token, "GET", &uri, Value::Null).await),
        before
    );
    let outcomes = futures_util::future::join_all((0..8).map(|_| f.policy(true, 0))).await;
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.0 == StatusCode::OK)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.0 == StatusCode::CONFLICT)
            .count(),
        7
    );
    let audits: Vec<(String, Value, Value)> = sqlx::query_as(
        "SELECT reason, before_json, after_json FROM admin_audit_logs \
         WHERE admin_id = ? AND target_type = 'agent' AND target_id = ? AND action = 'agent_recharge.authorize'",
    ).bind(f.admin).bind(f.agent.agent_id.to_string()).fetch_all(&f.pool).await.unwrap();
    assert_eq!(audits.len(), 1);
    assert!(!audits[0].0.is_empty());
    assert_eq!(audits[0].1["direct_recharge_enabled"], false);
    assert_eq!(audits[0].2["direct_recharge_enabled"], true);
    assert_eq!(audits[0].2["version"], 1);
    assert_eq!(f.counts().await, (0, 0, 0, 0, 0));
}

#[tokio::test]
async fn agent_recharge_financial_audit_accepts_credit_and_flags_amount_or_status_mismatch() {
    let Some(f) = RechargeFixture::new().await else {
        return;
    };
    let audit_uri = "/admin/api/v1/governance/financial-idempotency";
    let baseline = ok(request(&f.app, &f.admin_token, "GET", audit_uri, Value::Null).await);
    let original_orphans = baseline["orphan_ledger_entries"].as_i64().unwrap();
    let pending = f.pending("governance-valid").await;
    let credited_id = pending["id"].as_u64().unwrap();
    ok(f.review(credited_id, "approve", "governance fixture").await);
    f.assert_credit(credited_id, "1.23").await;
    let before_read = f.counts().await;
    let balance = f.balance(f.user).await;
    let valid = ok(request(&f.app, &f.admin_token, "GET", audit_uri, Value::Null).await);
    assert_eq!(valid["orphan_ledger_entries"], original_orphans);
    assert_eq!(f.counts().await, before_read);
    assert_eq!(f.balance(f.user).await, balance);

    let ledger_id: u64 = sqlx::query_scalar(
        "SELECT id FROM wallet_ledger WHERE user_id = ? AND asset_id = ? \
         AND ref_type = 'agent_recharge' AND ref_id = ?",
    )
    .bind(f.user)
    .bind(f.asset)
    .bind(credited_id.to_string())
    .fetch_one(&f.pool)
    .await
    .unwrap();
    let invalid_reference = f.pending("governance-not-credited").await;
    let invalid_id = invalid_reference["id"].as_u64().unwrap();
    for (label, amount, ref_id) in [
        ("amount", "1.24", credited_id),
        ("pending", "1.23", invalid_id),
        ("rejected", "1.23", invalid_id),
    ] {
        if label == "rejected" {
            ok(
                f.review(invalid_id, "reject", "governance rejected fixture")
                    .await,
            );
        }
        let before_read = f.counts().await;
        sqlx::query("UPDATE wallet_ledger SET amount = ?, ref_id = ? WHERE id = ?")
            .bind(decimal(amount))
            .bind(ref_id.to_string())
            .bind(ledger_id)
            .execute(&f.pool)
            .await
            .unwrap();
        let response = request(&f.app, &f.admin_token, "GET", audit_uri, Value::Null).await;
        // Restore only this fixture before asserting, even if the route rejects the query.
        sqlx::query("UPDATE wallet_ledger SET amount = ?, ref_id = ? WHERE id = ?")
            .bind(decimal("1.23"))
            .bind(credited_id.to_string())
            .bind(ledger_id)
            .execute(&f.pool)
            .await
            .unwrap();
        let anomaly = ok(response);
        assert_eq!(
            anomaly["orphan_ledger_entries"],
            original_orphans + 1,
            "{label}: {anomaly}"
        );
        assert_eq!(anomaly["status"], "attention", "{label}: {anomaly}");
        assert_eq!(f.counts().await, before_read);
        assert_eq!(f.balance(f.user).await, balance);
        let restored = ok(request(&f.app, &f.admin_token, "GET", audit_uri, Value::Null).await);
        assert_eq!(
            restored["orphan_ledger_entries"], original_orphans,
            "{label}"
        );
    }
    f.assert_credit(credited_id, "1.23").await;
}
