use std::fs;
use std::path::Path;

use aes_telex::film::{
    FILM_V1_PREAMBLE, FilmLimits, FilmStream, decode_film, decode_film_with_limits, encode_film,
    encode_film_with_limits, film_to_telex, telex_to_film,
};
use aes_telex::{
    COMPLETE_AES_PROFILE, ClarifierKind, DatatypeClarifier, DatatypeDescriptor, GenericArgument,
    PARTIAL_AES_PROFILE, TelexLimits, TelexRecord,
};
use serde_json::Value;

#[test]
fn film_default_empty_stream_has_exact_bytes() {
    let stream = FilmStream {
        profile: COMPLETE_AES_PROFILE.to_owned(),
        profile_explicit: false,
        projection: None,
        projection_explicit: false,
        records: Vec::new(),
    };
    let encoded = encode_film(&stream, &[]).expect("empty Film must encode");
    assert_eq!(encoded, [FILM_V1_PREAMBLE.as_slice(), &[0x00]].concat());
    assert_eq!(
        decode_film(&encoded, &[]).expect("empty Film must decode"),
        stream
    );
}

#[test]
fn film_encoder_checks_complete_size_before_allocating_an_empty_stream() {
    let stream = FilmStream {
        profile: COMPLETE_AES_PROFILE.to_owned(),
        profile_explicit: false,
        projection: None,
        projection_explicit: false,
        records: Vec::new(),
    };
    let limits = FilmLimits {
        max_input_bytes: FILM_V1_PREAMBLE.len(),
        ..FilmLimits::default()
    };
    let error = encode_film_with_limits(&stream, &[], &limits, &TelexLimits::default())
        .expect_err("the context byte must exceed the complete-stream byte limit");
    assert_eq!(error.code, "FILM_LIMIT_EXCEEDED");
    assert_eq!(error.component, "stream");
}

#[test]
fn film_encoder_bounds_its_payload_size_plan_before_allocation() {
    let stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: vec![
            TelexRecord::new(vec![
                ("path".to_owned(), "$.a".to_owned()),
                ("kind".to_owned(), "BooleanLiteral".to_owned()),
                ("value".to_owned(), "true".to_owned()),
            ]),
            TelexRecord::new(vec![
                ("path".to_owned(), "$.b".to_owned()),
                ("kind".to_owned(), "BooleanLiteral".to_owned()),
                ("value".to_owned(), "false".to_owned()),
            ]),
        ],
    };
    let sizing_plan_bytes = stream.records.len() * std::mem::size_of::<usize>();
    let limits = FilmLimits {
        max_buffered_bytes: sizing_plan_bytes - 1,
        ..FilmLimits::default()
    };
    let error = encode_film_with_limits(&stream, &[], &limits, &TelexLimits::default())
        .expect_err("the payload-size plan must remain within the encoder scratch limit");
    assert_eq!(error.code, "FILM_LIMIT_EXCEEDED");
    assert_eq!(error.component, "sizing-plan");
}

#[test]
fn complex_record_round_trips_without_telex_reconstruction() {
    let descriptor = DatatypeDescriptor {
        datatype: "list".to_owned(),
        generics: vec![GenericArgument::Datatype(DatatypeDescriptor {
            datatype: "int".to_owned(),
            generics: Vec::new(),
            clarifiers: Vec::new(),
        })],
        clarifiers: vec![DatatypeClarifier {
            kind: ClarifierKind::StringLiteral,
            value: ".".to_owned(),
        }],
    };
    let record = TelexRecord::with_datatype(
        vec![
            ("path".to_owned(), "$.items[0]".to_owned()),
            ("kind".to_owned(), "NumberLiteral".to_owned()),
            ("datatype".to_owned(), "list".to_owned()),
            ("identity".to_owned(), "binding-0".to_owned()),
            ("value".to_owned(), "42".to_owned()),
            (
                "origin".to_owned(),
                "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_owned(),
            ),
            ("span".to_owned(), "10:20".to_owned()),
            ("x.example.note".to_owned(), "café".to_owned()),
        ],
        descriptor,
    );
    let stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: vec![record],
    };
    let encoded = encode_film(&stream, &["x.example.note"]).expect("Film must encode");
    let decoded =
        decode_film(&encoded, &["x.example.note"]).expect("Film must decode and validate");
    assert_eq!(decoded, stream);
    assert_eq!(
        encode_film(&decoded, &["x.example.note"]).expect("Film must re-encode"),
        encoded
    );
}

#[test]
fn logical_datatype_fields_are_not_encoded_as_extensions() {
    let descriptor = DatatypeDescriptor {
        datatype: "list".to_owned(),
        generics: vec![GenericArgument::Datatype(DatatypeDescriptor {
            datatype: "int".to_owned(),
            generics: Vec::new(),
            clarifiers: Vec::new(),
        })],
        clarifiers: vec![DatatypeClarifier {
            kind: ClarifierKind::StringLiteral,
            value: ".".to_owned(),
        }],
    };
    let record = TelexRecord::with_datatype(
        vec![
            ("path".to_owned(), "$.items".to_owned()),
            ("kind".to_owned(), "ListNode".to_owned()),
            ("datatype".to_owned(), "list".to_owned()),
            ("generics".to_owned(), "[int]".to_owned()),
            ("clarifiers".to_owned(), "[\".\"]".to_owned()),
        ],
        descriptor,
    );
    let stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: vec![record],
    };

    let encoded = encode_film(&stream, &[]).expect("logical datatype fields must be core fields");
    let decoded = decode_film(&encoded, &[]).expect("encoded datatype must decode");
    assert!(decoded.records[0].datatype().is_some());
    assert_eq!(decoded.records[0].get("generics"), None);
    assert_eq!(decoded.records[0].get("clarifiers"), None);
}

#[test]
fn all_assigned_kind_codes_round_trip() {
    let kinds = [
        "StringLiteral",
        "NumberLiteral",
        "InfinityLiteral",
        "NaNLiteral",
        "NullLiteral",
        "BooleanLiteral",
        "ToggleLiteral",
        "HexLiteral",
        "RadixLiteral",
        "EncodingLiteral",
        "SeparatorLiteral",
        "SansaAddressLiteral",
        "DateLiteral",
        "TimeLiteral",
        "DateTimeLiteral",
        "WTCDateTimeLiteral",
        "ObjectNode",
        "ListNode",
        "TupleLiteral",
        "NodeLiteral",
        "NodeHead",
        "CloneReference",
        "PointerReference",
    ];
    let values = [
        "text",
        "1",
        "Infinity",
        "NaN",
        "notSet",
        "true",
        "yes",
        "cafe",
        "ff",
        "payload",
        ".",
        "$.source",
        "2025-01-01",
        "09:30",
        "2025-01-01T09:30",
        "2025-01-01T09:30&local",
        "",
        "",
        "",
        "",
        "tag",
        "$.source",
        "$.source",
    ];
    let records = kinds
        .iter()
        .zip(values)
        .enumerate()
        .map(|(index, (kind, value))| {
            let mut fields = vec![
                ("path".to_owned(), format!("$.item{index}")),
                ("kind".to_owned(), (*kind).to_owned()),
            ];
            if !value.is_empty()
                || !["ObjectNode", "ListNode", "TupleLiteral", "NodeLiteral"].contains(kind)
            {
                fields.push(("value".to_owned(), value.to_owned()));
            }
            TelexRecord::new(fields)
        })
        .collect::<Vec<_>>();
    let stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records,
    };
    let encoded = encode_film(&stream, &[]).expect("all kinds must encode");
    let decoded = decode_film(&encoded, &[]).expect("all kinds must decode");
    assert_eq!(decoded, stream);
}

#[test]
fn telex_transcoding_preserves_explicit_context_and_order() {
    let telex = concat!(
        "telex.aes=1\n",
        "profile=aes.partial.v1\n",
        "projection=aeon.document.v1\n\n",
        "header=$.[\"aeon:mode\"]\n",
        "kind=StringLiteral\n",
        "value=strict\n\n",
        "path=$.answer\n",
        "kind=NumberLiteral\n",
        "value=42\n"
    );
    let film = telex_to_film(telex, &[]).expect("Telex must transcode");
    assert_eq!(
        film_to_telex(&film, &[]).expect("Film must transcode"),
        telex
    );
}

#[test]
fn malformed_or_noncanonical_frames_fail_closed() {
    let cases = [
        (&b"O__\xff"[..], "FILM_TRUNCATED"),
        (&b"BAD!!\x00"[..], "FILM_INVALID_PREAMBLE"),
        (&b"O__\xff\x01"[..], "FILM_TRUNCATED"),
        (&b"O__\xff\x01\x80\x00"[..], "FILM_INVALID_CONTEXT"),
        (&b"O__\xff\x01\x04"[..], "FILM_INVALID_CONTEXT"),
        (&b"O__\xff\x01\x00\x00"[..], "FILM_NONCANONICAL"),
        (&b"O__\xff\x01\x00\x01\x00"[..], "FILM_TRUNCATED"),
    ];
    for (input, code) in cases {
        let error = decode_film(input, &[]).expect_err("input must fail");
        assert_eq!(error.code, code, "input: {input:?}");
    }
}

#[test]
fn film_limits_apply_before_record_decode() {
    let stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: vec![TelexRecord::new(vec![
            ("path".to_owned(), "$.value".to_owned()),
            ("kind".to_owned(), "StringLiteral".to_owned()),
            ("value".to_owned(), "payload".to_owned()),
        ])],
    };
    let encoded = encode_film(&stream, &[]).expect("Film must encode");
    let limits = FilmLimits {
        max_record_bytes: 1,
        ..FilmLimits::default()
    };
    let error = decode_film_with_limits(&encoded, &[], &limits, &TelexLimits::default())
        .expect_err("oversized record must fail");
    assert_eq!(error.code, "FILM_LIMIT_EXCEEDED");
    assert_eq!(error.component, "record");
}

#[test]
fn unknown_extensions_require_registration() {
    let stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: vec![TelexRecord::new(vec![
            ("path".to_owned(), "$.value".to_owned()),
            ("kind".to_owned(), "StringLiteral".to_owned()),
            ("value".to_owned(), "payload".to_owned()),
            ("x.example.claim".to_owned(), "yes".to_owned()),
        ])],
    };
    let error =
        encode_film(&stream, &[]).expect_err("unregistered extension must fail semantic encoding");
    assert_eq!(error.code, "FILM_AES_INVALID");
    encode_film(&stream, &["x.example.claim"]).expect("registered extension must encode");
}

#[test]
fn released_telex_semantic_examples_round_trip_through_film() {
    let suite_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/telex/v1/suites/02-event-and-profile-validation.json");
    let suite: Value = serde_json::from_str(
        &fs::read_to_string(suite_path).expect("Telex CTS suite must be readable"),
    )
    .expect("Telex CTS suite must be JSON");
    let mut exercised = 0_usize;
    for vector in suite["tests"]
        .as_array()
        .expect("Telex CTS tests must be an array")
    {
        if vector["operation"] != "validate" || vector["expected"]["valid"] != true {
            continue;
        }
        let id = vector["id"].as_str().expect("vector id must be a string");
        let telex = vector["input"]["telex"]
            .as_str()
            .expect("Telex input must be a string");
        let registered = vector["input"]["registered_fields"]
            .as_array()
            .map(|fields| {
                fields
                    .iter()
                    .map(|field| field.as_str().expect("registered field must be a string"))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let film = telex_to_film(telex, &registered)
            .unwrap_or_else(|error| panic!("{id}: Film encoding failed: {error}"));
        let round_trip = film_to_telex(&film, &registered)
            .unwrap_or_else(|error| panic!("{id}: Film decoding failed: {error}"));
        assert_eq!(round_trip, telex, "{id}");
        exercised = exercised.saturating_add(1);
    }
    assert_eq!(exercised, 11, "unexpected valid Telex CTS example count");
}
