use aes_telex::film::{
    FilmLimits, FilmStream, decode_film_view_with_limits, decode_film_with_limits, encode_film,
    encode_film_with_limits,
};
use aes_telex::film_candidate_b::encode_film_candidate_b;
use aes_telex::film_candidate_c::encode_film_candidate_c;
use aes_telex::{
    AesCanonicalPath, AesEventAddress, AesEventRecord, AesValueKind, DatatypeDescriptor,
    GenericArgument, TelexLimits, TelexRecord, encode_aes_event_records_with_projection_and_limits,
    encode_telex, parse_datatype_descriptor,
};

#[test]
fn typed_and_extensible_writers_reject_literal_generic_arguments() {
    let limits = TelexLimits::default();
    for literal in ["2", "9007199254740993", "2.0000000000000001"] {
        for nested in [false, true] {
            let literal_argument = GenericArgument::NumberLiteral(literal.to_owned());
            let argument = if nested {
                GenericArgument::Datatype(DatatypeDescriptor {
                    datatype: "nested".to_owned(),
                    generics: vec![literal_argument],
                    clarifiers: Vec::new(),
                })
            } else {
                literal_argument
            };
            let datatype = DatatypeDescriptor {
                datatype: "custom".to_owned(),
                generics: vec![argument],
                clarifiers: Vec::new(),
            };
            let record = TelexRecord::with_datatype(
                vec![
                    ("path".to_owned(), "$.a".to_owned()),
                    ("kind".to_owned(), "NumberLiteral".to_owned()),
                    ("value".to_owned(), "0".to_owned()),
                ],
                datatype.clone(),
            );
            assert!(encode_telex(std::slice::from_ref(&record), None).is_err());
            let typed = AesEventRecord {
                address: AesEventAddress::Path(
                    AesCanonicalPath::parse("$.a".to_owned()).expect("path"),
                ),
                kind: AesValueKind::NumberLiteral,
                datatype: Some(datatype),
                identity: None,
                value: Some("0".to_owned()),
                origin: None,
                span: None,
            };
            assert!(
                encode_aes_event_records_with_projection_and_limits(&[typed], None, None, &limits)
                    .is_err()
            );
            let stream = FilmStream {
                profile: "aes.complete.v1".to_owned(),
                profile_explicit: false,
                projection: None,
                projection_explicit: false,
                records: vec![record],
            };
            assert!(encode_film(&stream, &[]).is_err());
            assert!(encode_film_candidate_b(&stream, &[]).is_err());
            assert!(encode_film_candidate_c(&stream, &[]).is_err());
        }
    }
    for argument in ["2", "1.0", "-2", "1e3", "\"int\"", "nested<2>"] {
        assert!(parse_datatype_descriptor(&format!("custom<{argument}>"), &limits).is_err());
    }
}

#[test]
fn legacy_film_views_cannot_cross_a_validated_boundary() {
    let suite: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/film/v1/suites/05-next-type-only-generics.json"
    ))
    .expect("suite");
    let mut checked = 0;
    for fixture in suite["tests"].as_array().expect("tests") {
        if fixture["operation"] != "decode"
            || !fixture["expected"]["error"]["diagnostic_codes"]
                .as_array()
                .is_some_and(|codes| codes.iter().any(|code| code == "AES_INVALID_DATATYPE"))
        {
            continue;
        }
        let hex = fixture["input"]["film_hex"].as_str().expect("hex");
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("byte"))
            .collect::<Vec<_>>();
        let limits = TelexLimits {
            max_generic_arguments: 2,
            max_clarifier_values: 2,
            ..TelexLimits::default()
        };
        let registered = fixture["input"]["registered_fields"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|name| name.as_str().expect("field"))
            .collect::<Vec<_>>();
        let view = decode_film_view_with_limits(&bytes, &FilmLimits::default(), &limits)
            .expect("physical framing remains recognizable");
        let error = view
            .to_validated_owned(&registered, &limits)
            .expect_err("view is not accepted AES");
        assert_eq!(error.code, "FILM_AES_INVALID");
        assert!(
            error
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "AES_INVALID_DATATYPE")
        );
        assert!(
            view.to_validated_owned_path_arena(&registered, &limits)
                .is_err()
        );
        assert!(
            decode_film_with_limits(&bytes, &registered, &FilmLimits::default(), &limits).is_err()
        );
        assert!(
            encode_film_with_limits(
                &view.to_owned_unvalidated(),
                &registered,
                &FilmLimits::default(),
                &limits
            )
            .is_err()
        );
        checked += 1;
    }
    assert_eq!(checked, 8);
}
