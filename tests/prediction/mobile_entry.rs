use super::*;

#[tokio::test]
async fn mobile_entry_visibility_round_trips_with_audit_and_legacy_preservation()
-> Result<(), Box<dyn Error>> {
    let _guard = PREDICTION_CONFIG_GOVERNANCE_LOCK.lock().await;
    let Some(pool) = mysql_pool().await else {
        return Ok(());
    };
    let original = sqlx::query_as::<_, PredictionSettingsGovernanceSnapshot>(
        "SELECT * FROM prediction_settings WHERE id = 1",
    )
    .fetch_one(&pool)
    .await?;
    let (role_id, admin_id) = create_prediction_admin(&pool).await;
    let suffix = Uuid::now_v7().simple().to_string();
    let asset_id = sqlx::query(
        "INSERT INTO assets (symbol, name, precision_scale, asset_type, status) VALUES (?, 'Mobile entry test', 8, 'coin', 'active')",
    )
    .bind(format!("ME{}", &suffix[..12]))
    .execute(&pool)
    .await?
    .last_insert_id();
    sqlx::query(
        "INSERT INTO prediction_asset_configs (asset_id, enabled, max_payout_amount) VALUES (?, TRUE, 1000)",
    )
    .bind(asset_id)
    .execute(&pool)
    .await?;
    let settings = test_settings();
    let token = issue_token(
        &settings,
        format!("admin:{admin_id}"),
        TokenScope::Admin,
        900,
    )?;
    let app = build_router(AppState::new(settings).with_mysql(pool.clone()));
    let outcome: Result<(), Box<dyn Error>> = async {
        let mut revision = original.revision;
        for (index, enabled) in [false, true].into_iter().enumerate() {
            let mut body = prediction_settings_write_body(asset_id, revision, "调整手机端显示", 300);
            body["mobile_entry_enabled"] = json!(enabled);
            // Both public response branches must carry the saved flag.
            if enabled {
                body["allowed_asset_ids"] = json!([]);
            }
            let (status, _, saved) = write_prediction_admin_config(
                app.clone(), "PATCH", "/admin/api/v1/prediction/settings", &token,
                &format!("mobile-entry-{index}"), body.clone(),
            ).await?;
            assert_eq!(status, StatusCode::OK, "{saved}");
            assert_eq!(saved["mobile_entry_enabled"], enabled);
            assert_eq!(saved["sync_enabled"], true);
            revision += 1;
            assert_eq!(saved["revision"], revision);

            let read = app.clone().oneshot(
                Request::builder().uri("/admin/api/v1/prediction/settings")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::empty())?,
            ).await?;
            assert_eq!(read.status(), StatusCode::OK);
            let read: Value = serde_json::from_slice(&axum::body::to_bytes(read.into_body(), 1_048_576).await?)?;
            assert_eq!(read["mobile_entry_enabled"], enabled);

            let public = app.clone().oneshot(
                Request::builder().uri("/api/v1/prediction/config").body(Body::empty())?,
            ).await?;
            assert_eq!(public.status(), StatusCode::OK);
            let public: Value = serde_json::from_slice(&axum::body::to_bytes(public.into_body(), 1_048_576).await?)?;
            assert_eq!(public["mobile_entry_enabled"], enabled);
            assert_eq!(public["allowed_assets"].as_array().unwrap().len(), usize::from(!enabled));

            body["mobile_entry_enabled"] = json!(!enabled);
            let (status, _, _) = write_prediction_admin_config(
                app.clone(), "PATCH", "/admin/api/v1/prediction/settings", &token,
                &format!("mobile-entry-stale-{index}"), body.clone(),
            ).await?;
            assert_eq!(status, StatusCode::CONFLICT);

            body.as_object_mut().unwrap().remove("mobile_entry_enabled");
            body["revision"] = json!(revision);
            let (status, _, saved) = write_prediction_admin_config(
                app.clone(), "PATCH", "/admin/api/v1/prediction/settings", &token,
                &format!("mobile-entry-legacy-{index}"), body,
            ).await?;
            assert_eq!(status, StatusCode::OK, "{saved}");
            assert_eq!(saved["mobile_entry_enabled"], enabled);
            revision += 1;
        }

        let audits = sqlx::query_as::<_, PredictionConfigAuditRow>(
            "SELECT action, before_json, after_json, reason, ip, request_id FROM admin_audit_logs WHERE admin_id = ? AND target_type = 'prediction_settings' ORDER BY id",
        ).bind(admin_id).fetch_all(&pool).await?;
        assert_eq!(audits.len(), 4, "stale updates must not write audit records");
        assert_eq!(audits[0].before_json.0["mobile_entry_enabled"], original.mobile_entry_enabled);
        assert_eq!(audits[0].after_json.0["mobile_entry_enabled"], false);
        assert_eq!(audits[1].after_json.0["mobile_entry_enabled"], false);
        assert_eq!(audits[2].before_json.0["mobile_entry_enabled"], false);
        assert_eq!(audits[2].after_json.0["mobile_entry_enabled"], true);
        assert_eq!(audits[3].after_json.0["mobile_entry_enabled"], true);
        Ok(())
    }.await;

    restore_prediction_settings(&pool, &original).await?;
    sqlx::query("DELETE FROM admin_audit_logs WHERE admin_id = ?")
        .bind(admin_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM prediction_asset_configs WHERE asset_id = ?")
        .bind(asset_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM assets WHERE id = ?")
        .bind(asset_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM admin_users WHERE id = ?")
        .bind(admin_id)
        .execute(&pool)
        .await?;
    sqlx::query("DELETE FROM admin_roles WHERE id = ?")
        .bind(role_id)
        .execute(&pool)
        .await?;
    outcome
}
