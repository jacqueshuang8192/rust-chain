use super::*;

fn config(allowed: &[&str], rules: &[&str]) -> KycConfigValidationInput {
    KycConfigValidationInput {
        enabled: true,
        target_kyc_level: 1,
        required_documents: vec!["identity_front".into(), "identity_back".into()],
        allowed_countries: allowed.iter().map(|value| (*value).into()).collect(),
        country_document_types: rules
            .iter()
            .map(|country| KycCountryDocumentTypeRule {
                country: (*country).into(),
                document_types: vec!["passport".into()],
                handheld_document_types: vec![],
            })
            .collect(),
        max_document_size_bytes: 5 * 1024 * 1024,
    }
}

#[test]
fn active_country_policy_rejects_disjoint_or_incomplete_document_rules() {
    for input in [config(&["CN"], &["US"]), config(&["CN", "US"], &["CN"])] {
        assert!(
            matches!(validate_kyc_config(input), Err(AppError::Validation(message)) if message == "allowed_countries must have matching country_document_types")
        );
    }
}

#[test]
fn country_policy_keeps_empty_restriction_semantics() {
    for input in [config(&[], &[]), config(&["CN"], &[]), config(&[], &["CN"])] {
        assert!(validate_kyc_config(input).is_ok());
    }
}

#[test]
fn country_policy_preserves_legacy_identity_and_accepts_extra_inactive_rules() {
    let result =
        validate_kyc_config(config(&[" China ", "中国"], &["CHINA", "中国", "Japan"])).unwrap();
    assert_eq!(result.allowed_countries, vec!["China", "中国"]);
    assert_eq!(result.country_document_types[0].country, "CHINA");
    assert_eq!(result.country_document_types.len(), 3);
    assert!(validate_kyc_config(config(&["CN"], &["China"])).is_err());
}

#[test]
fn conflicting_legacy_policy_can_still_be_disabled() {
    let mut input = config(&["CN"], &["US"]);
    input.enabled = false;
    assert!(!validate_kyc_config(input).unwrap().enabled);
}
