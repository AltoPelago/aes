use aes_telex::{
    AEON_DOCUMENT_PROJECTION, check_prefix_completeness, check_telex_completeness, parse_telex,
};

#[test]
fn reports_missing_prefixes_without_full_profile_validation() {
    let result = check_telex_completeness(
        "telex.aes=1\nprofile=aes.partial.v1\n\npath=$.a.b\nkind=NumberLiteral\nvalue=1\n",
    )
    .expect("valid Telex must be checkable");

    assert!(!result.complete);
    assert_eq!(result.missing.len(), 1);
    assert_eq!(result.missing[0].field, None);
    assert_eq!(result.missing[0].path, "$.a");
    assert_eq!(result.missing[0].required_by, "$.a.b");
}

#[test]
fn keeps_header_and_body_address_planes_disjoint() {
    let parsed = parse_telex(
        "telex.aes=1\nprojection=aeon.document.v1\nprofile=aes.partial.v1\n\nheader=$.[\"aeon:mode\"].name\nkind=StringLiteral\nvalue=strict\n\npath=$.name\nkind=StringLiteral\nvalue=value\n",
    )
    .expect("fixture must parse");
    let result = check_prefix_completeness(&parsed.records, Some(AEON_DOCUMENT_PROJECTION))
        .expect("valid projection must be checkable");

    assert!(!result.complete);
    assert_eq!(result.missing.len(), 1);
    assert_eq!(result.missing[0].field, Some("header"));
    assert_eq!(result.missing[0].path, "$.[\"aeon:mode\"]");
}
