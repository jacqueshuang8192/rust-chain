use super::*;
use exchange_api::modules::auth::{TokenScope, issue_token};
use serde_json::json;
use sqlx::mysql::MySqlPoolOptions;

#[tokio::test]
async fn home_carousel_openapi_and_anonymous_boundaries() {
    let doc = openapi_json().await;
    let public = &doc["paths"]["/api/v1/home/carousel"]["get"];
    assert!(public.is_object());
    assert!(public["security"].is_null() || public["security"] == json!([]));
    for method in ["get", "patch"] {
        assert!(operation_has_bearer_security(
            &doc,
            "/admin/api/v1/home/carousel",
            method
        ));
    }
    let schemas = &doc["components"]["schemas"];
    assert_eq!(
        schemas["PublicHomeCarouselResponse"]["properties"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    assert!(
        schemas["SaveHomeCarouselRequest"]["required"]
            .as_array()
            .unwrap()
            .contains(&json!("expected_revision"))
    );
    let app = build_router(test_state());
    for method in ["GET", "PATCH"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri("/admin/api/v1/home/carousel")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"expected_revision":0,"slides":[],"reason":"test"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/home/carousel")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

async fn request(
    app: &axum::Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(builder.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

fn slide(id: &str, position: u8, locale: Option<&str>, enabled: bool, target: Value) -> Value {
    json!({"id":id,"position":position,"enabled":enabled,"locale":locale,
        "title":"邀请好友","description":"查看活动详情","image_url":"https://cdn.example.test/a.png","target":target})
}

#[tokio::test]
async fn home_carousel_mysql_migration_filtering_revision_permissions_and_audit() {
    let Ok(url) = std::env::var("HOME_CAROUSEL_TEST_DATABASE_URL") else {
        eprintln!("SKIPPED: HOME_CAROUSEL_TEST_DATABASE_URL is not set");
        return;
    };
    let parsed = url::Url::parse(&url).unwrap();
    assert_eq!(
        parsed.host_str(),
        Some("127.0.0.1"),
        "isolated local database only"
    );
    assert!(parsed.path().ends_with("_test"));
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let settings = (*test_state().settings).clone();
    let mut tokens = Vec::new();
    for permission in ["content.news.*", "content.news.read"] {
        let name = uuid::Uuid::now_v7().to_string();
        let role = sqlx::query("INSERT INTO admin_roles (name, permissions) VALUES (?, ?)")
            .bind(&name)
            .bind(sqlx::types::Json(json!([permission])))
            .execute(&pool)
            .await
            .unwrap()
            .last_insert_id();
        let admin = sqlx::query(
            "INSERT INTO admin_users (username,password_hash,role_id) VALUES (?,'test',?)",
        )
        .bind(&name)
        .bind(role)
        .execute(&pool)
        .await
        .unwrap()
        .last_insert_id();
        tokens.push(
            issue_token(&settings, format!("admin:{admin}"), TokenScope::Admin, 900).unwrap(),
        );
    }
    let mut news_ids = Vec::new();
    for status in ["published", "draft"] {
        news_ids.push(sqlx::query("INSERT INTO admin_news_items (title,category,status,default_locale,content_json) VALUES ('test','promotion',?,'zh-CN',JSON_OBJECT())")
            .bind(status).execute(&pool).await.unwrap().last_insert_id());
    }
    let app = build_router(AppState::new(settings).with_mysql(pool.clone()));
    let admin_path = "/admin/api/v1/home/carousel";
    let (status, before) = request(&app, "GET", admin_path, Some(&tokens[0]), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{before}");
    let revision = before["revision"].as_u64().unwrap();
    let slides = vec![
        slide("invite", 0, None, true, json!({"type":"referrals"})),
        slide("disabled", 1, None, false, json!({"type":"news_center"})),
        slide(
            "english",
            2,
            Some("en-US"),
            true,
            json!({"type":"news_center"}),
        ),
        slide(
            "published",
            3,
            Some("zh-CN"),
            true,
            json!({"type":"news_detail","id":news_ids[0]}),
        ),
        slide(
            "draft",
            4,
            None,
            true,
            json!({"type":"news_detail","id":news_ids[1]}),
        ),
    ];
    let body = json!({"expected_revision":revision,"reason":"publish carousel","slides":slides});
    let (status, _) = request(&app, "PATCH", admin_path, Some(&tokens[1]), body.clone()).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, saved) = request(&app, "PATCH", admin_path, Some(&tokens[0]), body.clone()).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    assert_eq!(saved["revision"], revision + 1);
    assert_eq!(
        request(&app, "PATCH", admin_path, Some(&tokens[0]), body)
            .await
            .0,
        StatusCode::CONFLICT
    );
    let (_, zh) = request(
        &app,
        "GET",
        "/api/v1/home/carousel?locale=zh",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(
        zh["slides"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["invite", "published"]
    );
    let (_, en) = request(
        &app,
        "GET",
        "/api/v1/home/carousel?locale=en",
        None,
        Value::Null,
    )
    .await;
    assert_eq!(en["slides"].as_array().unwrap().len(), 2);
    assert_eq!(
        request(
            &app,
            "GET",
            "/api/v1/home/carousel?locale=../zh",
            None,
            Value::Null
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    sqlx::query("UPDATE admin_news_items SET status='draft' WHERE id=?")
        .bind(news_ids[0])
        .execute(&pool)
        .await
        .unwrap();
    let (_, hidden) = request(&app, "GET", "/api/v1/home/carousel", None, Value::Null).await;
    assert_eq!(hidden["slides"].as_array().unwrap().len(), 1);
    let audit: (sqlx::types::Json<Value>,sqlx::types::Json<Value>,String) = sqlx::query_as(
        "SELECT before_json,after_json,reason FROM admin_audit_logs WHERE action='home_carousel.save' ORDER BY id DESC LIMIT 1"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(audit.0.0["slides"], before["slides"]);
    assert_eq!(audit.1.0, saved);
    assert_eq!(audit.2, "publish carousel");
    let empty = json!({"expected_revision":revision+1,"slides":[],"reason":"clear"});
    assert_eq!(
        request(&app, "PATCH", admin_path, Some(&tokens[0]), empty)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "GET", "/api/v1/home/carousel", None, Value::Null)
            .await
            .1,
        json!({"slides":[]})
    );
    let concurrent = json!({"expected_revision":revision+2,"slides":[],"reason":"concurrent"});
    let (first, second) = tokio::join!(
        request(
            &app,
            "PATCH",
            admin_path,
            Some(&tokens[0]),
            concurrent.clone()
        ),
        request(&app, "PATCH", admin_path, Some(&tokens[0]), concurrent)
    );
    let mut statuses = [first.0.as_u16(), second.0.as_u16()];
    statuses.sort();
    assert_eq!(statuses, [200, 409]);
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM admin_audit_logs WHERE action='home_carousel.save'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    // Inject an audit failure to verify the preceding config write rolls back.
    sqlx::raw_sql("CREATE TRIGGER home_carousel_fail_audit BEFORE INSERT ON admin_audit_logs FOR EACH ROW SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='local audit failure'")
        .execute(&pool).await.unwrap();
    let failed = request(
        &app,
        "PATCH",
        admin_path,
        Some(&tokens[0]),
        json!({"expected_revision":revision+3,"slides":[],"reason":"rollback probe"}),
    )
    .await;
    sqlx::raw_sql("DROP TRIGGER home_carousel_fail_audit")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(failed.0, StatusCode::INTERNAL_SERVER_ERROR);
    let (_, after_failure) = request(&app, "GET", admin_path, Some(&tokens[0]), Value::Null).await;
    assert_eq!(after_failure["revision"], revision + 3);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM admin_audit_logs WHERE action='home_carousel.save'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        audit_count
    );
    assert!(
        sqlx::query("INSERT INTO home_carousel_configs(id,slides) VALUES (2,JSON_OBJECT())")
            .execute(&pool)
            .await
            .is_err()
    );
}
