use aes_telex::{
    AesCanonicalPath, AesEventAddress, AesEventRecord, AesValueKind, ClarifierKind,
    DatatypeClarifier, DatatypeDescriptor, TelexLimits, TelexRecord,
    encode_aes_event_records_with_projection_and_limits, encode_telex_with_projection_and_limits,
    validate_aes_event_records_with_projection_and_limits,
    validate_telex_records_with_projection_and_limits,
};

fn equivalent_records() -> (Vec<TelexRecord>, Vec<AesEventRecord>) {
    let datatype = DatatypeDescriptor {
        datatype: "Number".to_owned(),
        generics: Vec::new(),
        clarifiers: vec![DatatypeClarifier {
            kind: ClarifierKind::NumberLiteral,
            value: "2".to_owned(),
        }],
    };
    let telex = vec![TelexRecord::with_datatype(
        vec![
            ("path".to_owned(), "$.answer".to_owned()),
            ("kind".to_owned(), "NumberLiteral".to_owned()),
            ("datatype".to_owned(), String::new()),
            ("identity".to_owned(), "answer-id".to_owned()),
            ("value".to_owned(), "42".to_owned()),
        ],
        datatype.clone(),
    )];
    let typed = vec![AesEventRecord {
        address: AesEventAddress::Path(
            AesCanonicalPath::parse("$.answer".to_owned()).expect("canonical test path"),
        ),
        kind: AesValueKind::NumberLiteral,
        datatype: Some(datatype),
        identity: Some("answer-id".to_owned()),
        value: Some("42".to_owned()),
        origin: None,
        span: None,
    }];
    (telex, typed)
}

#[test]
fn typed_records_match_extensible_records_for_validation_and_encoding() {
    let limits = TelexLimits::default();
    let (telex, typed) = equivalent_records();

    assert_eq!(
        validate_aes_event_records_with_projection_and_limits(
            &typed,
            "aes.complete.v1",
            None,
            &limits,
        ),
        validate_telex_records_with_projection_and_limits(
            &telex,
            "aes.complete.v1",
            None,
            &[],
            &limits,
        ),
    );
    assert_eq!(
        encode_aes_event_records_with_projection_and_limits(
            &typed,
            Some("aes.complete.v1"),
            None,
            &limits,
        ),
        encode_telex_with_projection_and_limits(&telex, Some("aes.complete.v1"), None, &limits,),
    );
}

#[test]
fn typed_records_retain_semantic_and_resource_validation() {
    let record = AesEventRecord {
        address: AesEventAddress::Path(
            AesCanonicalPath::parse("$.bad".to_owned()).expect("canonical test path"),
        ),
        kind: AesValueKind::HexLiteral,
        datatype: None,
        identity: None,
        value: Some("ABC".to_owned()),
        origin: None,
        span: None,
    };
    let validation = validate_aes_event_records_with_projection_and_limits(
        std::slice::from_ref(&record),
        "aes.complete.v1",
        None,
        &TelexLimits::default(),
    );
    assert_eq!(validation.diagnostics[0].code, "AES_INVALID_VALUE");

    let limits = TelexLimits {
        max_events: 0,
        ..TelexLimits::default()
    };
    let error = encode_aes_event_records_with_projection_and_limits(
        &[record],
        Some("aes.complete.v1"),
        None,
        &limits,
    )
    .expect_err("typed encoding must enforce the common event limit");
    assert_eq!(error.code, "TELEX_LIMIT_EXCEEDED");
    assert_eq!(error.counter, Some("max_events"));
}

#[test]
fn segment_builder_matches_validated_canonical_paths_and_is_copy_on_write() {
    let mut path = AesCanonicalPath::root();
    path.push_member("answer").expect("nonempty member");
    path.push_index(12);
    path.push_attribute("not bare").expect("nonempty attribute");

    let parsed = AesCanonicalPath::parse("$.answer[12].@.[\"not bare\"]".to_owned())
        .expect("builder output is canonical");
    assert_eq!(path, parsed);

    let mut child = path.clone();
    child.push_member("leaf").expect("nonempty member");
    assert_eq!(path.as_str(), "$.answer[12].@.[\"not bare\"]");
    assert_eq!(child.as_str(), "$.answer[12].@.[\"not bare\"].leaf");
}
