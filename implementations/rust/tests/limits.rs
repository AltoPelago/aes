use aes_telex::{
    TelexLimits, TelexRecord, encode_telex_with_limits, parse_telex_with_limits,
    validate_telex_records_with_limits,
};

#[test]
fn publishes_altopelago_telex_limit_defaults() {
    assert_eq!(
        TelexLimits::default(),
        TelexLimits {
            max_input_bytes: 67_108_864,
            max_line_bytes: 1_048_576,
            max_fields_per_event: 64,
            max_events: 100_000,
            max_decoded_payload_bytes: 33_554_432,
            max_path_depth: 1_024,
            max_path_characters: 8_192,
            max_attribute_depth: 1,
            max_value_nesting_depth: 256,
            max_string_codepoints: 1_048_576,
            max_key_segment_codepoints: 1_024,
            max_list_items: 65_536,
            max_tuple_items: 65_536,
            max_generic_depth: 1,
            max_generic_arguments: 32,
            max_clarifier_values: 1,
            max_datatype_components: 64,
        }
    );
}

#[test]
fn reports_structured_limit_exhaustion() {
    let limits = TelexLimits {
        max_input_bytes: 11,
        ..TelexLimits::default()
    };
    let error = parse_telex_with_limits("telex.aes=1\n", &limits)
        .expect_err("input should exceed its configured byte limit");
    assert_eq!(error.code, "TELEX_LIMIT_EXCEEDED");
    assert_eq!(error.counter, Some("max_input_bytes"));
    assert_eq!(error.observed, Some(12));
    assert_eq!(error.limit, Some(11));
}

#[test]
fn encoder_enforces_shared_structural_limits() {
    let records = vec![TelexRecord::new(vec![
        ("path".to_owned(), "$.a.@.x.@.y".to_owned()),
        ("kind".to_owned(), "NumberLiteral".to_owned()),
        ("value".to_owned(), "1".to_owned()),
    ])];
    let limits = TelexLimits {
        max_attribute_depth: 1,
        ..TelexLimits::default()
    };
    let error = encode_telex_with_limits(&records, Some("aes.partial.v1"), &limits)
        .expect_err("encode should enforce the selected structural limit");
    assert_eq!(error.code, "TELEX_LIMIT_EXCEEDED");
    assert_eq!(error.counter, Some("max_attribute_depth"));
}

#[test]
fn represented_limit_diagnostics_follow_parent_record_order() {
    let records = [
        ("$.a", "ListNode", None),
        ("$.a[0]", "NumberLiteral", Some("0")),
        ("$.a[1]", "NumberLiteral", Some("1")),
        ("$.b", "ListNode", None),
        ("$.b[0]", "NumberLiteral", Some("0")),
        ("$.b[1]", "NumberLiteral", Some("1")),
    ]
    .into_iter()
    .map(|(path, kind, value)| {
        let mut fields = vec![
            ("path".to_owned(), path.to_owned()),
            ("kind".to_owned(), kind.to_owned()),
        ];
        if let Some(value) = value {
            fields.push(("value".to_owned(), value.to_owned()));
        }
        TelexRecord::new(fields)
    })
    .collect::<Vec<_>>();
    let limits = TelexLimits {
        max_list_items: 1,
        ..TelexLimits::default()
    };

    let result = validate_telex_records_with_limits(&records, "aes.complete.v1", &[], &limits);
    let records = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.counter == Some("max_list_items"))
        .map(|diagnostic| diagnostic.record)
        .collect::<Vec<_>>();
    assert_eq!(records, [Some(0), Some(3)]);
}
