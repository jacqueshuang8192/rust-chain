//! 注册法律文件的公开解析、管理员成组发布和注册版本确认。

use crate::{
    error::{AppError, AppResult},
    modules::{
        auth::{
            infrastructure::legal as store,
            presentation::legal::{
                AdminLegalResponse, PublishRegistrationLegalRequest, RegistrationLegalBundle,
                UserRegistrationLegalReceipt,
            },
        },
        countries::normalize_country_code,
    },
};
use axum::http::StatusCode;
use chrono::Utc;
use sqlx::{MySql, Pool, Transaction};

fn locale(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.len() < 2
        || value.len() > 16
        || value.starts_with('-')
        || value.ends_with('-')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(AppError::Validation("invalid legal document locale".into()));
    }
    Ok(value.to_owned())
}

fn scope(value: &str) -> AppResult<String> {
    if value.trim().eq_ignore_ascii_case("GLOBAL") {
        return Ok("GLOBAL".into());
    }
    normalize_country_code(value)
}

fn validate_publication(request: &mut PublishRegistrationLegalRequest) -> AppResult<()> {
    request.scope_code = scope(&request.scope_code)?;
    request.locale = locale(&request.locale)?;
    request.reason = request.reason.trim().to_owned();
    if request.reason.is_empty() || request.reason.chars().count() > 500 {
        return Err(AppError::Validation(
            "publication reason is required (max 500 characters)".into(),
        ));
    }
    if request.effective_on > Utc::now().date_naive() {
        return Err(AppError::Validation(
            "legal document effective date cannot be in the future".into(),
        ));
    }
    for (title, body) in [
        (&mut request.terms_title, &mut request.terms_body),
        (&mut request.privacy_title, &mut request.privacy_body),
    ] {
        *title = title.trim().to_owned();
        *body = body.trim().to_owned();
        if title.is_empty()
            || title.chars().count() > 160
            || body.chars().count() < 80
            || body.len() > 131_072
        {
            return Err(AppError::Validation(
                "legal document title/body length is invalid".into(),
            ));
        }
    }
    Ok(())
}

/// 公开读取指定国家当前完整协议组；只读取发布版本，不把缺少正文伪装成默认合法文案。
pub(crate) async fn public_registration_legal(
    pool: &Pool<MySql>,
    country_code: &str,
    requested_locale: Option<&str>,
) -> AppResult<RegistrationLegalBundle> {
    let country_code = normalize_country_code(country_code)?;
    let default_locale = store::country_default_locale(pool, &country_code).await?;
    let requested = locale(requested_locale.unwrap_or(&default_locale))?;
    let mut tx = pool.begin().await?;
    let result = store::resolve_publication_in_tx(
        &mut tx,
        &country_code,
        &default_locale,
        &requested,
        false,
    )
    .await?
    .ok_or_else(|| AppError::Conflict("该地区尚未发布用户协议和隐私政策".into()))?;
    tx.commit().await?;
    Ok(result)
}

/// 在注册事务内再次核对最新整组版本；未确认、旧版本或错国家均不消耗验证码或创建账号。
pub(crate) async fn require_current_registration_legal(
    tx: &mut Transaction<'_, MySql>,
    country_code: &str,
    default_locale: &str,
    requested_locale: Option<&str>,
    publication_id: Option<u64>,
    acknowledged: Option<bool>,
) -> AppResult<RegistrationLegalBundle> {
    if acknowledged != Some(true) || publication_id.is_none_or(|id| id == 0) {
        return Err(AppError::Validation(
            "请先阅读并确认用户协议和隐私政策".into(),
        ));
    }
    let requested = locale(requested_locale.unwrap_or(default_locale))?;
    let current =
        store::resolve_publication_in_tx(tx, country_code, default_locale, &requested, true)
            .await?
            .ok_or_else(|| AppError::Conflict("该地区尚未发布用户协议和隐私政策".into()))?;
    if publication_id != Some(current.publication_id) {
        return Err(AppError::Api {
            status: StatusCode::CONFLICT,
            code: "LEGAL_DOCUMENTS_STALE",
            message: "协议已更新，请重新阅读并确认".into(),
        });
    }
    Ok(current)
}

/// 记录注册所用的精确正文主键，事务所有权仍属于注册用例。
pub(crate) async fn store_registration_legal_receipt(
    tx: &mut Transaction<'_, MySql>,
    user_id: u64,
    country_code: &str,
    bundle: &RegistrationLegalBundle,
) -> AppResult<()> {
    store::insert_receipt_in_tx(tx, user_id, country_code, bundle).await
}

/// 管理员只看精确范围的现行版本和最近30次发布，不通过公开回退规则混淆正在编辑的范围。
pub(crate) async fn admin_registration_legal(
    pool: &Pool<MySql>,
    scope_code: &str,
    requested_locale: &str,
) -> AppResult<AdminLegalResponse> {
    let scope_code = scope(scope_code)?;
    let locale = locale(requested_locale)?;
    let mut tx = pool.begin().await?;
    let current = store::load_published_in_tx(&mut tx, &scope_code, &locale, false).await?;
    tx.commit().await?;
    let editions = store::list_editions(pool, &scope_code, &locale).await?;
    Ok(AdminLegalResponse { current, editions })
}

/// 管理员成组发布两个不可变版本；地区覆盖只允许已有国家，版本冲突不产生半发布。
pub(crate) async fn publish_registration_legal(
    pool: &Pool<MySql>,
    admin_id: u64,
    mut request: PublishRegistrationLegalRequest,
) -> AppResult<RegistrationLegalBundle> {
    validate_publication(&mut request)?;
    if request.scope_code != "GLOBAL" {
        store::country_default_locale(pool, &request.scope_code).await?;
    }
    let mut tx = pool.begin().await?;
    if request.scope_code != "GLOBAL" {
        store::lock_legal_country_in_tx(&mut tx, &request.scope_code).await?;
    }
    let result = store::publish_in_tx(&mut tx, admin_id, &request).await?;
    tx.commit().await?;
    Ok(result)
}

/// 只读管理员历史正文供版本复核，匿名用户永远看不到未发布编辑态。
pub(crate) async fn admin_legal_publication_by_id(
    pool: &Pool<MySql>,
    id: u64,
) -> AppResult<RegistrationLegalBundle> {
    let mut tx = pool.begin().await?;
    let result = store::load_publication_in_tx(&mut tx, id).await?;
    tx.commit().await?;
    Ok(result)
}

/// 管理员按用户ID读取不可变确认凭据；缺失用户或旧用户无记录返回404。
pub(crate) async fn admin_legal_receipt(
    pool: &Pool<MySql>,
    user_id: u64,
) -> AppResult<UserRegistrationLegalReceipt> {
    store::load_receipt(pool, user_id)
        .await?
        .ok_or(AppError::NotFound)
}
