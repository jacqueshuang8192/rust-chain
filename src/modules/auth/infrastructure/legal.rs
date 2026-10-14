//! 注册法律文件的不可变版本和当前指针；纯文本与受控富文本分格式保存。

use crate::{
    error::{AppError, AppResult},
    modules::{
        admin::infrastructure::{AdminAuditLogEntry, insert_admin_audit_log_entry_in_tx},
        auth::presentation::legal::{
            PublishRegistrationLegalRequest, RegistrationLegalBody, RegistrationLegalBundle,
            RegistrationLegalDocument, RegistrationLegalEdition, UserRegistrationLegalReceipt,
        },
    },
};
use chrono::NaiveDate;
use sqlx::{MySql, Pool, Transaction};

#[derive(sqlx::FromRow)]
struct PublicationRow {
    id: u64,
    published_at: chrono::DateTime<chrono::Utc>,
    scope_code: String,
    locale: String,
    edition: u64,
    terms_id: u64,
    privacy_id: u64,
}

#[derive(sqlx::FromRow)]
struct DocumentRow {
    id: u64,
    scope_code: String,
    locale: String,
    kind: String,
    title: String,
    body_text: String,
    body_format: String,
    effective_on: NaiveDate,
}

/// 返回国家配置默认语言；注册准入仍由注册事务内原有的国家行锁判定。
pub(crate) async fn country_default_locale(
    pool: &Pool<MySql>,
    country_code: &str,
) -> AppResult<String> {
    sqlx::query_scalar("SELECT default_locale FROM country_configs WHERE country_code = ? LIMIT 1")
        .bind(country_code)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::Validation("country_code is not configured".into()))
}

/// 地区文件首次发布前锁定其国家配置；与注册用例的国家→发布指针锁序一致。
/// 国家在编辑间隙被删除时返回校验错误，避免为不存在的范围发布覆盖版本。
pub(crate) async fn lock_legal_country_in_tx(
    tx: &mut Transaction<'_, MySql>,
    country_code: &str,
) -> AppResult<()> {
    let exists: Option<u64> = sqlx::query_scalar(
        "SELECT id FROM country_configs WHERE country_code=? LIMIT 1 FOR UPDATE",
    )
    .bind(country_code)
    .fetch_optional(&mut **tx)
    .await?;
    if exists.is_none() {
        return Err(AppError::Validation(
            "country_code is not configured".into(),
        ));
    }
    Ok(())
}

/// 按统一优先级找已成组发布的两份文件；注册时锁当前指针直至账号与确认记录一起提交。
/// 仅返回有效发布行；正文的种类/范围/语言若与指针不符则报错，禁止拼接两次发布的内容。
pub(crate) async fn resolve_publication_in_tx(
    tx: &mut Transaction<'_, MySql>,
    country_code: &str,
    default_locale: &str,
    requested_locale: &str,
    lock_current: bool,
) -> AppResult<Option<RegistrationLegalBundle>> {
    let language = requested_locale
        .split('-')
        .next()
        .unwrap_or(requested_locale);
    let options = [
        (country_code, requested_locale),
        (country_code, language),
        ("GLOBAL", requested_locale),
        ("GLOBAL", language),
        (country_code, default_locale),
        ("GLOBAL", default_locale),
        ("GLOBAL", "en"),
    ];
    for (scope, locale) in options {
        if let Some(bundle) = load_published_in_tx(tx, scope, locale, lock_current).await? {
            return Ok(Some(bundle));
        }
    }
    Ok(None)
}

/// 按精确地区和语言读取当前发布版本；管理端编辑器不自动回退到统一版本。
pub(crate) async fn load_published_in_tx(
    tx: &mut Transaction<'_, MySql>,
    scope: &str,
    locale: &str,
    lock_current: bool,
) -> AppResult<Option<RegistrationLegalBundle>> {
    let query = if lock_current {
        "SELECT publication_id FROM registration_legal_current WHERE scope_code=? AND locale=? AND publication_id IS NOT NULL FOR SHARE"
    } else {
        "SELECT publication_id FROM registration_legal_current WHERE scope_code=? AND locale=? AND publication_id IS NOT NULL"
    };
    let id: Option<u64> = sqlx::query_scalar(query)
        .bind(scope)
        .bind(locale)
        .fetch_optional(&mut **tx)
        .await?;
    let Some(id) = id else { return Ok(None) };
    load_publication_in_tx(tx, id).await.map(Some)
}

/// 用不可变版本主键加载已发布正文；历史版本仅在管理员鉴权路径上供复核。
pub(crate) async fn load_publication_in_tx(
    tx: &mut Transaction<'_, MySql>,
    id: u64,
) -> AppResult<RegistrationLegalBundle> {
    let row = sqlx::query_as::<_, PublicationRow>(
        "SELECT id,published_at,scope_code,locale,edition,terms_id,privacy_id FROM registration_legal_publications WHERE id=?",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)?;
    let terms = load_document(tx, row.terms_id, &row.scope_code, &row.locale, "terms").await?;
    let privacy =
        load_document(tx, row.privacy_id, &row.scope_code, &row.locale, "privacy").await?;
    Ok(RegistrationLegalBundle {
        publication_id: row.id,
        published_at: row.published_at,
        scope_code: row.scope_code,
        locale: row.locale,
        edition: row.edition,
        terms,
        privacy,
    })
}

async fn load_document(
    tx: &mut Transaction<'_, MySql>,
    id: u64,
    scope: &str,
    locale: &str,
    kind: &str,
) -> AppResult<RegistrationLegalDocument> {
    let row = sqlx::query_as::<_, DocumentRow>(
        "SELECT id,scope_code,locale,kind,title,body_text,body_format,effective_on FROM registration_legal_documents WHERE id=?",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    if row.scope_code != scope || row.locale != locale || row.kind != kind {
        return Err(AppError::Internal("法律文件发布指针与正文不匹配".into()));
    }
    let body = match row.body_format.as_str() {
        "plain" => RegistrationLegalBody::Plain(row.body_text),
        "rich_json" => match serde_json::from_str(&row.body_text)
            .map_err(|_| AppError::Internal("法律文件富文本存储格式错误".into()))?
        {
            RegistrationLegalBody::Rich(blocks) => RegistrationLegalBody::Rich(blocks),
            RegistrationLegalBody::Plain(_) => {
                return Err(AppError::Internal("法律文件富文本存储格式错误".into()));
            }
        },
        _ => return Err(AppError::Internal("法律文件正文格式不受支持".into())),
    };
    Ok(RegistrationLegalDocument {
        id: row.id,
        title: row.title,
        body,
        effective_on: row.effective_on,
    })
}

/// 同事务追加两份不可变正文、成组发布记录、当前指针和管理员审计。
/// 编辑竞争由当前行锁与 expected_edition 阻断；任何插入失败整体回滚。
pub(crate) async fn publish_in_tx(
    tx: &mut Transaction<'_, MySql>,
    admin_id: u64,
    request: &PublishRegistrationLegalRequest,
) -> AppResult<RegistrationLegalBundle> {
    sqlx::query("INSERT IGNORE INTO registration_legal_current(scope_code,locale) VALUES (?,?)")
        .bind(&request.scope_code)
        .bind(&request.locale)
        .execute(&mut **tx)
        .await?;
    let old_edition: u64 = sqlx::query_scalar(
        "SELECT edition FROM registration_legal_current WHERE scope_code=? AND locale=? FOR UPDATE",
    )
    .bind(&request.scope_code)
    .bind(&request.locale)
    .fetch_one(&mut **tx)
    .await?;
    if old_edition != request.expected_edition {
        return Err(AppError::Conflict(
            "法律文件已由其他管理员更新，请刷新后重试".into(),
        ));
    }
    let edition = old_edition
        .checked_add(1)
        .ok_or_else(|| AppError::Validation("legal document edition exhausted".into()))?;
    let terms_id = insert_document(
        tx,
        request,
        admin_id,
        "terms",
        &request.terms_title,
        &request.terms_body,
    )
    .await?;
    let privacy_id = insert_document(
        tx,
        request,
        admin_id,
        "privacy",
        &request.privacy_title,
        &request.privacy_body,
    )
    .await?;
    let publication_id = sqlx::query(
        "INSERT INTO registration_legal_publications(scope_code,locale,edition,terms_id,privacy_id,published_by) VALUES (?,?,?,?,?,?)",
    )
    .bind(&request.scope_code)
    .bind(&request.locale)
    .bind(edition)
    .bind(terms_id)
    .bind(privacy_id)
    .bind(admin_id)
    .execute(&mut **tx)
    .await?
    .last_insert_id();
    sqlx::query("UPDATE registration_legal_current SET edition=?,publication_id=? WHERE scope_code=? AND locale=?")
        .bind(edition)
        .bind(publication_id)
        .bind(&request.scope_code)
        .bind(&request.locale)
        .execute(&mut **tx)
        .await?;
    insert_admin_audit_log_entry_in_tx(
        tx,
        admin_id,
        AdminAuditLogEntry {
            action: "registration_legal.publish",
            target_type: "registration_legal",
            target_id: publication_id,
            before_json: Some(serde_json::json!({"edition": old_edition})),
            after_json: Some(serde_json::json!({
                "scope_code": request.scope_code,
                "locale": request.locale,
                "edition": edition,
                "terms_id": terms_id,
                "privacy_id": privacy_id
            })),
            reason: Some(request.reason.clone()),
        },
    )
    .await?;
    load_publication_in_tx(tx, publication_id).await
}

async fn insert_document(
    tx: &mut Transaction<'_, MySql>,
    request: &PublishRegistrationLegalRequest,
    admin_id: u64,
    kind: &str,
    title: &str,
    body: &RegistrationLegalBody,
) -> AppResult<u64> {
    let (body_format, body_text) = match body {
        RegistrationLegalBody::Plain(text) => ("plain", text.clone()),
        RegistrationLegalBody::Rich(_) => (
            "rich_json",
            serde_json::to_string(body)
                .map_err(|_| AppError::Internal("法律文件序列化失败".into()))?,
        ),
    };
    Ok(sqlx::query("INSERT INTO registration_legal_documents(scope_code,locale,kind,title,body_text,body_format,effective_on,published_by) VALUES (?,?,?,?,?,?,?,?)")
        .bind(&request.scope_code).bind(&request.locale).bind(kind).bind(title).bind(body_text).bind(body_format)
        .bind(request.effective_on).bind(admin_id).execute(&mut **tx).await?.last_insert_id())
}

/// 保存注册时两份正文的精确版本；同用户唯一，失败必须由注册事务整体回滚。
pub(crate) async fn insert_receipt_in_tx(
    tx: &mut Transaction<'_, MySql>,
    user_id: u64,
    country_code: &str,
    bundle: &RegistrationLegalBundle,
) -> AppResult<()> {
    sqlx::query("INSERT INTO user_registration_legal_receipts(user_id,country_code,scope_code,locale,publication_id,terms_id,privacy_id) VALUES (?,?,?,?,?,?,?)")
        .bind(user_id).bind(country_code).bind(&bundle.scope_code).bind(&bundle.locale)
        .bind(bundle.publication_id).bind(bundle.terms.id).bind(bundle.privacy.id)
        .execute(&mut **tx).await?;
    Ok(())
}

/// 只返回管理员限定地区/语言最近的发布版本元数据，正文需读取指定版本。
pub(crate) async fn list_editions(
    pool: &Pool<MySql>,
    scope: &str,
    locale: &str,
) -> AppResult<Vec<RegistrationLegalEdition>> {
    Ok(sqlx::query_as("SELECT id,edition,published_at,published_by FROM registration_legal_publications WHERE scope_code=? AND locale=? ORDER BY edition DESC LIMIT 30")
        .bind(scope).bind(locale).fetch_all(pool).await?)
}

/// 管理员根据用户 ID 查询注册确认记录，不向匿名接口暴露用户身份。
pub(crate) async fn load_receipt(
    pool: &Pool<MySql>,
    user_id: u64,
) -> AppResult<Option<UserRegistrationLegalReceipt>> {
    sqlx::query_as("SELECT user_id,country_code,scope_code,locale,publication_id,terms_id,privacy_id,acknowledged_at FROM user_registration_legal_receipts WHERE user_id=?")
        .bind(user_id).fetch_optional(pool).await.map_err(AppError::from)
}
