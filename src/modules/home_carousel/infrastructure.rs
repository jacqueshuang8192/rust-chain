use super::domain::{HomeCarouselDocument, HomeCarouselTargetType};
use crate::{
    error::{AppError, AppResult},
    modules::home_carousel::domain::HomeCarouselSlide,
};
use serde_json::Value;
use sqlx::{MySql, Pool, Transaction, types::Json as SqlxJson};

/// 轮播单例的版本与 JSON 原文，仅供应用层解码。
#[derive(Debug, sqlx::FromRow)]
pub struct HomeCarouselRecord {
    pub revision: u64,
    pub slides: SqlxJson<Value>,
}

/// 读取已迁移创建的单例；行缺失时返回错误，禁止默默重建覆盖。
pub async fn load_home_carousel(pool: &Pool<MySql>) -> AppResult<HomeCarouselRecord> {
    sqlx::query_as::<_, HomeCarouselRecord>(
        "SELECT revision, slides FROM home_carousel_configs WHERE id = 1",
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}

/// 在调用方事务内取得排他行锁，供应用层串行执行版本比较与审计。
pub async fn lock_home_carousel(tx: &mut Transaction<'_, MySql>) -> AppResult<HomeCarouselRecord> {
    sqlx::query_as::<_, HomeCarouselRecord>(
        "SELECT revision, slides FROM home_carousel_configs WHERE id = 1 FOR UPDATE",
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound)
}

/// 更新调用方已锁定的单例；不自行提交，保证配置和审计一同回滚。
pub async fn update_home_carousel(
    tx: &mut Transaction<'_, MySql>,
    revision: u64,
    document: &HomeCarouselDocument,
    admin_id: u64,
) -> AppResult<()> {
    sqlx::query(
        r#"UPDATE home_carousel_configs
           SET revision = ?, slides = ?, updated_by_admin_id = ?, updated_at = CURRENT_TIMESTAMP(6)
           WHERE id = 1"#,
    )
    .bind(revision)
    .bind(SqlxJson(serde_json::to_value(document).map_err(|_| {
        AppError::Internal("failed to serialize home carousel".to_owned())
    })?))
    .bind(admin_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// 保留配置顺序，仅公开启用且语言匹配的条目；新闻详情逐项核验实时发布状态。
pub async fn filter_public_slides(
    pool: &Pool<MySql>,
    document: HomeCarouselDocument,
    locale: Option<&str>,
) -> AppResult<Vec<HomeCarouselSlide>> {
    let requested_locale = locale.unwrap_or("zh-CN");
    let mut slides = Vec::new();
    for slide in document.slides {
        if !slide.enabled
            || slide
                .locale
                .as_deref()
                .is_some_and(|slide_locale| !locale_matches(slide_locale, requested_locale))
        {
            continue;
        }
        if matches!(slide.target.target_type, HomeCarouselTargetType::NewsDetail) {
            let news_id = slide.target.id.ok_or_else(|| {
                AppError::Internal("home carousel news target is missing id".to_owned())
            })?;
            let published = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM admin_news_items WHERE id = ? AND status = 'published')",
            )
            .bind(news_id)
            .fetch_one(pool)
            .await?;
            if !published {
                continue;
            }
        }
        slides.push(slide);
    }
    slides.sort_by_key(|slide| slide.position);
    Ok(slides)
}

fn locale_matches(slide_locale: &str, requested_locale: &str) -> bool {
    let slide_family = slide_locale.split('-').next().unwrap_or(slide_locale);
    let requested_family = requested_locale
        .split('-')
        .next()
        .unwrap_or(requested_locale);
    slide_locale.eq_ignore_ascii_case(requested_locale)
        || slide_family.eq_ignore_ascii_case(requested_family)
}
