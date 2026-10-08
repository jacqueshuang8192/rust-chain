use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// 单份首页配置最多保存十二页，包含未启用项。
pub const MAX_SLIDES: usize = 12;
/// 标题按 Unicode 字符计数。
pub const MAX_TITLE_LENGTH: usize = 80;
/// 描述按 Unicode 字符计数。
pub const MAX_DESCRIPTION_LENGTH: usize = 180;
/// 图片地址的存储上限。
pub const MAX_IMAGE_URL_LENGTH: usize = 2048;
/// 数字 ID 和配置版本必须能被浏览器无损表示。
pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// 仅允许客户端声明的站内目的地，不接受任意外链。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum HomeCarouselTargetType {
    NewsDetail,
    NewsCenter,
    Referrals,
}

/// 新闻详情必须带正整数 ID，其他目的地不携带 ID。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HomeCarouselTarget {
    #[serde(rename = "type")]
    pub target_type: HomeCarouselTargetType,
    pub id: Option<u64>,
}

/// 后台可编辑的轮播条目；公开前还需核验启用状态、语言和目标可见性。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HomeCarouselSlide {
    pub id: String,
    pub position: u8,
    pub enabled: bool,
    pub locale: Option<String>,
    pub title: String,
    pub description: String,
    pub image_url: String,
    pub target: HomeCarouselTarget,
}

/// 单例配置持久化文档，不含管理员身份或版本元数据。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HomeCarouselDocument {
    pub slides: Vec<HomeCarouselSlide>,
}

/// 保存前校验全部条目并规范化文本；错误时整份拒绝，不做部分保存。
pub fn validate_document(mut document: HomeCarouselDocument) -> AppResult<HomeCarouselDocument> {
    if document.slides.len() > MAX_SLIDES {
        return Err(AppError::Validation(format!(
            "home carousel supports at most {MAX_SLIDES} slides"
        )));
    }

    let mut ids = HashSet::with_capacity(document.slides.len());
    for slide in &mut document.slides {
        let id = &slide.id;
        if id.is_empty()
            || id.len() > 64
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
        {
            return Err(AppError::Validation(
                "home carousel slide id is invalid".to_owned(),
            ));
        }
        if !ids.insert(id.to_owned()) {
            return Err(AppError::Validation(
                "home carousel slide ids must be unique".to_owned(),
            ));
        }
        if slide.position as usize >= MAX_SLIDES {
            return Err(AppError::Validation(
                "home carousel slide position is invalid".to_owned(),
            ));
        }
        validate_text(&slide.title, MAX_TITLE_LENGTH, "home carousel title")?;
        validate_text(
            &slide.description,
            MAX_DESCRIPTION_LENGTH,
            "home carousel description",
        )?;
        validate_image_url(&slide.image_url)?;
        slide.title = slide.title.trim().to_owned();
        slide.description = slide.description.trim().to_owned();
        slide.image_url = slide.image_url.trim().to_owned();
        if let Some(locale) = &slide.locale {
            slide.locale = Some(validate_locale(locale)?);
        }
        match slide.target.target_type {
            HomeCarouselTargetType::NewsDetail
                if !slide
                    .target
                    .id
                    .is_some_and(|id| (1..=MAX_SAFE_INTEGER).contains(&id)) =>
            {
                return Err(AppError::Validation(
                    "news_detail target requires a safe positive id".to_owned(),
                ));
            }
            HomeCarouselTargetType::NewsCenter | HomeCarouselTargetType::Referrals
                if slide.target.id.is_some() =>
            {
                return Err(AppError::Validation(
                    "this home carousel target must not include id".to_owned(),
                ));
            }
            HomeCarouselTargetType::NewsDetail
            | HomeCarouselTargetType::NewsCenter
            | HomeCarouselTargetType::Referrals => {}
        }
    }

    let positions = document
        .slides
        .iter()
        .map(|slide| slide.position)
        .collect::<Vec<_>>();
    if positions.windows(2).any(|window| window[0] >= window[1]) {
        return Err(AppError::Validation(
            "home carousel slide positions must be strictly increasing".to_owned(),
        ));
    }

    Ok(document)
}

/// 公开查询与配置保存共用同一语言格式；仅规范化两端空白。
pub fn validate_locale(value: &str) -> AppResult<String> {
    let value = value.trim();
    let valid = matches!(value.len(), 2 | 5)
        && value.bytes().enumerate().all(|(index, byte)| {
            if index == 2 && value.len() == 5 {
                byte == b'-'
            } else if value.len() == 5 && index > 2 {
                byte.is_ascii_uppercase()
            } else {
                byte.is_ascii_lowercase()
            }
        });
    if !valid {
        return Err(AppError::Validation(
            "home carousel locale is invalid".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

fn validate_text(value: &str, max_length: usize, field: &str) -> AppResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > max_length {
        return Err(AppError::Validation(format!(
            "{field} must be 1-{max_length} characters"
        )));
    }
    Ok(())
}

fn validate_image_url(value: &str) -> AppResult<()> {
    let trimmed = value.trim();
    let parsed = url::Url::parse(trimmed)
        .map_err(|_| AppError::Validation("home carousel image_url is invalid".to_owned()))?;
    if trimmed.chars().count() > MAX_IMAGE_URL_LENGTH
        || !matches!(parsed.scheme(), "https" | "http")
        || parsed.host_str().is_none()
        || parsed.username() != ""
        || parsed.password().is_some()
    {
        return Err(AppError::Validation(
            "home carousel image_url must be a public http(s) URL".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/unit_src/src_modules_home_carousel_domain_tests.rs"]
mod tests;
