use super::{
    domain::{HomeCarouselDocument, MAX_SAFE_INTEGER, validate_document, validate_locale},
    infrastructure::{
        filter_public_slides, load_home_carousel, lock_home_carousel, update_home_carousel,
    },
    presentation::{
        AdminHomeCarouselResponse, PublicHomeCarouselResponse, SaveHomeCarouselRequest,
    },
};
use crate::{
    architecture::ApplicationLayer,
    error::{AppError, AppResult},
    modules::admin::infrastructure::{AdminAuditLogEntry, insert_admin_audit_log_entry_in_tx},
};
use serde_json::json;
use sqlx::{MySql, Pool};

/// 负责首页配置读取、公开过滤以及版本锁定保存的用例边界。
#[derive(Debug)]
pub struct HomeCarouselUseCase;

impl ApplicationLayer for HomeCarouselUseCase {}

/// 先校验查询语言，再读取配置并排除禁用、语言不匹配或未发布新闻；不暴露审计字段。
pub async fn get_public_home_carousel(
    pool: &Pool<MySql>,
    locale: Option<String>,
) -> AppResult<PublicHomeCarouselResponse> {
    let locale = locale.as_deref().map(validate_locale).transpose()?;
    let record = load_home_carousel(pool).await?;
    let document = decode_document(record.slides.0)?;
    Ok(PublicHomeCarouselResponse {
        slides: filter_public_slides(pool, document, locale.as_deref()).await?,
    })
}

/// 返回完整可编辑配置及版本；损坏的存储文档返回服务错误，不以空配置覆盖。
pub async fn get_admin_home_carousel(pool: &Pool<MySql>) -> AppResult<AdminHomeCarouselResponse> {
    let record = load_home_carousel(pool).await?;
    let document = decode_document(record.slides.0)?;
    Ok(AdminHomeCarouselResponse {
        revision: record.revision,
        slides: document.slides,
    })
}

/// 锁定单例行后比对期望版本；配置、版本和前后审计快照同事务提交，冲突不写入。
pub async fn save_admin_home_carousel(
    pool: &Pool<MySql>,
    admin_id: u64,
    request: SaveHomeCarouselRequest,
) -> AppResult<AdminHomeCarouselResponse> {
    let reason = request.reason.trim().to_owned();
    if reason.is_empty() || reason.chars().count() > 512 {
        return Err(AppError::Validation(
            "home carousel save reason must be 1-512 characters".to_owned(),
        ));
    }
    let expected_revision = request.expected_revision;
    if expected_revision >= MAX_SAFE_INTEGER {
        return Err(AppError::Validation(
            "home carousel revision is invalid".to_owned(),
        ));
    }
    let document = validate_document(request.into())?;
    let mut tx = pool.begin().await?;
    let before = lock_home_carousel(&mut tx).await?;
    if before.revision != expected_revision {
        return Err(AppError::Conflict(
            "home carousel revision changed; reload before saving".to_owned(),
        ));
    }
    let before_document = decode_document(before.slides.0)?;
    let next_revision = before
        .revision
        .checked_add(1)
        .ok_or_else(|| AppError::Conflict("home carousel revision overflow".to_owned()))?;
    update_home_carousel(&mut tx, next_revision, &document, admin_id).await?;
    insert_admin_audit_log_entry_in_tx(
        &mut tx,
        admin_id,
        AdminAuditLogEntry {
            action: "home_carousel.save",
            target_type: "home_carousel_config",
            target_id: 1,
            before_json: Some(json!({
                "revision": before.revision,
                "slides": before_document.slides,
            })),
            after_json: Some(json!({
                "revision": next_revision,
                "slides": document.slides,
            })),
            reason: Some(reason),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(AdminHomeCarouselResponse {
        revision: next_revision,
        slides: document.slides,
    })
}

fn decode_document(value: serde_json::Value) -> AppResult<HomeCarouselDocument> {
    serde_json::from_value(value)
        .map_err(|_| AppError::Internal("stored home carousel is invalid".to_owned()))
        .and_then(|document| {
            validate_document(document)
                .map_err(|_| AppError::Internal("stored home carousel is invalid".to_owned()))
        })
}
