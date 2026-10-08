use super::{
    HomeCarouselDocument, HomeCarouselSlide, HomeCarouselTarget, HomeCarouselTargetType,
    MAX_SLIDES, validate_document,
};

fn slide(id: &str, position: u8) -> HomeCarouselSlide {
    HomeCarouselSlide {
        id: id.to_owned(),
        position,
        enabled: true,
        locale: Some("zh-CN".to_owned()),
        title: "活动".to_owned(),
        description: "首页推广内容".to_owned(),
        image_url: "https://cdn.example.test/banner.png".to_owned(),
        target: HomeCarouselTarget {
            target_type: HomeCarouselTargetType::Referrals,
            id: None,
        },
    }
}

#[test]
fn validates_home_carousel_limits_targets_and_positions() {
    let slides = (0..MAX_SLIDES)
        .map(|position| slide(&format!("slide-{position}"), position as u8))
        .collect();
    assert!(validate_document(HomeCarouselDocument { slides }).is_ok());

    let mut too_many = vec![slide("slide-0", 0)];
    too_many.extend(
        (1..=MAX_SLIDES).map(|position| slide(&format!("slide-{position}"), position as u8)),
    );
    assert!(validate_document(HomeCarouselDocument { slides: too_many }).is_err());

    let mut duplicate = vec![slide("same", 0), slide("same", 1)];
    assert!(
        validate_document(HomeCarouselDocument {
            slides: duplicate.split_off(0)
        })
        .is_err()
    );

    let mut unordered = vec![slide("first", 1), slide("second", 0)];
    assert!(
        validate_document(HomeCarouselDocument {
            slides: unordered.split_off(0)
        })
        .is_err()
    );
}

#[test]
fn rejects_unknown_fields_and_invalid_target_shape() {
    let unknown = serde_json::json!({
        "slides": [{
            "id": "slide-0",
            "position": 0,
            "enabled": true,
            "locale": "zh-CN",
            "title": "活动",
            "description": "首页推广内容",
            "image_url": "https://cdn.example.test/banner.png",
            "target": {
                "type": "referrals",
                "id": null,
                "unexpected": true
            }
        }]
    });
    assert!(serde_json::from_value::<HomeCarouselDocument>(unknown).is_err());

    let mut invalid = slide("news", 0);
    invalid.target.target_type = HomeCarouselTargetType::NewsDetail;
    assert!(
        validate_document(HomeCarouselDocument {
            slides: vec![invalid]
        })
        .is_err()
    );
}

#[test]
fn normalizes_text_and_locales_but_rejects_ambiguous_ids() {
    let mut value = slide("invite", 0);
    value.title = "  活动  ".into();
    value.description = "  描述  ".into();
    value.locale = Some(" zh-CN ".into());
    value.image_url = " https://cdn.example.test/image.png ".into();
    let document = validate_document(HomeCarouselDocument {
        slides: vec![value],
    })
    .unwrap();
    assert_eq!(document.slides[0].title, "活动");
    assert_eq!(document.slides[0].description, "描述");
    assert_eq!(document.slides[0].locale.as_deref(), Some("zh-CN"));
    assert_eq!(
        document.slides[0].image_url,
        "https://cdn.example.test/image.png"
    );
    assert!(
        validate_document(HomeCarouselDocument {
            slides: vec![slide(" invite ", 0)]
        })
        .is_err()
    );
    for locale in ["", "zh_cn", "ZH", "en-us", "abc", "zh-CN-extra"] {
        assert!(super::validate_locale(locale).is_err(), "{locale}");
    }
}

#[test]
fn rejects_unsafe_news_ids_images_and_content_lengths() {
    for id in [
        None,
        Some(0),
        Some(super::MAX_SAFE_INTEGER + 1),
        Some(u64::MAX),
    ] {
        let mut value = slide("news", 0);
        value.target = HomeCarouselTarget {
            target_type: HomeCarouselTargetType::NewsDetail,
            id,
        };
        assert!(
            validate_document(HomeCarouselDocument {
                slides: vec![value]
            })
            .is_err()
        );
    }
    for url in [
        "javascript:alert(1)",
        "data:image/png,abc",
        "/relative.png",
        "https://user:pass@example.test/image",
    ] {
        let mut value = slide("invite", 0);
        value.image_url = url.into();
        assert!(
            validate_document(HomeCarouselDocument {
                slides: vec![value]
            })
            .is_err()
        );
    }
    for title in [" ".into(), "文".repeat(81)] {
        let mut value = slide("invite", 0);
        value.title = title;
        assert!(
            validate_document(HomeCarouselDocument {
                slides: vec![value]
            })
            .is_err()
        );
    }
}
