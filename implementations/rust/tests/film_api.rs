use aes_telex::film::{
    FilmAddressView, FilmGenericView, FilmLimits, FilmStream, decode_film, decode_film_view,
    decode_film_view_with_limits, encode_film,
};
use aes_telex::{
    ClarifierKind, DatatypeClarifier, DatatypeDescriptor, GenericArgument, PARTIAL_AES_PROFILE,
    TelexLimits, TelexRecord,
};

#[test]
fn borrowed_view_reuses_input_storage_and_materializes_explicitly() {
    let origin = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
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
            ("path".to_owned(), "$.item".to_owned()),
            ("kind".to_owned(), "NumberLiteral".to_owned()),
            ("datatype".to_owned(), "list".to_owned()),
            ("identity".to_owned(), "binding-0".to_owned()),
            ("value".to_owned(), "42".to_owned()),
            ("origin".to_owned(), origin.to_owned()),
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
    let view = decode_film_view(&encoded).expect("Film view must decode");

    assert_eq!(view.input_bytes(), encoded.len());
    assert_eq!(view.profile, PARTIAL_AES_PROFILE);
    assert!(borrows(&encoded, view.profile.as_bytes()));
    assert_eq!(view.records.len(), 1);
    let record = &view.records[0];
    assert_eq!(record.address, FilmAddressView::Path("$.item"));
    assert!(borrows(&encoded, record.address.value().as_bytes()));
    assert_eq!(record.kind, "NumberLiteral");
    assert_eq!(record.identity, Some("binding-0"));
    assert_eq!(record.value, Some("42"));
    assert!(borrows(
        &encoded,
        record
            .identity
            .expect("identity must be present")
            .as_bytes(),
    ));
    assert!(borrows(
        &encoded,
        record.value.expect("value must be present").as_bytes(),
    ));
    let datatype = record.datatype.as_ref().expect("datatype must be present");
    assert_eq!(datatype.datatype, "list");
    assert!(borrows(&encoded, datatype.datatype.as_bytes()));
    assert!(matches!(
        datatype.generics.as_slice(),
        [FilmGenericView::Datatype(nested)] if nested.datatype == "int"
    ));
    assert_eq!(datatype.clarifiers[0].value, ".");
    assert!(borrows(&encoded, datatype.clarifiers[0].value.as_bytes()));
    let origin_bytes = record.origin.expect("origin must be present");
    assert!(borrows(&encoded, origin_bytes));
    assert_eq!(record.span, Some((10, 20)));
    assert_eq!(record.extensions[0].name, "x.example.note");
    assert_eq!(record.extensions[0].value, "café");
    assert!(borrows(&encoded, record.extensions[0].value.as_bytes()));

    let owned = view
        .to_validated_owned(&["x.example.note"], &TelexLimits::default())
        .expect("materialized stream must validate");
    assert_eq!(owned, stream);
    assert_eq!(
        decode_film(&encoded, &["x.example.note"]).expect("owned Film must decode"),
        stream
    );
}

#[test]
fn borrowed_film_validity_remains_provisional_until_aes_validation() {
    let bytes = decode_hex("4f5f5fff01000f00010a6e6f742d612d706174680178");
    let view = decode_film_view(&bytes).expect("physical Film must decode");
    assert_eq!(view.records[0].address.value(), "not-a-path");

    let error = view
        .to_validated_owned(&[], &TelexLimits::default())
        .expect_err("invalid AES path must prevent completed materialization");
    assert_eq!(error.code, "FILM_AES_INVALID");
    assert_eq!(error.diagnostics[0].code, "AES_INVALID_PATH");
}

#[test]
fn borrowed_decode_enforces_datatype_components_before_materialization() {
    let bytes = decode_hex(concat!(
        "4f5f5fff01010e6165732e7061727469616c2e76316b1e020a242e6974656d735b305d",
        "19046c69737402000603696e7400000201330201012e020231300962696e64696e672d30",
        "0234320123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0a14",
        "0e782e6578616d706c652e6e6f746505636166c3a9",
    ));
    let limits = TelexLimits {
        max_datatype_components: 4,
        max_generic_arguments: 2,
        max_clarifier_values: 2,
        ..TelexLimits::default()
    };
    let error = decode_film_view_with_limits(&bytes, &FilmLimits::default(), &limits)
        .expect_err("syntax decoding must enforce shared structural limits");

    assert_eq!(error.code, "FILM_AES_INVALID");
    assert_eq!(error.component, "clarifier-count");
    assert_eq!(
        error.diagnostics[0].counter,
        Some("max_datatype_components")
    );
}

#[test]
fn owned_materialization_outlives_the_input_buffer() {
    let owned = {
        let stream = FilmStream {
            profile: PARTIAL_AES_PROFILE.to_owned(),
            profile_explicit: true,
            projection: None,
            projection_explicit: false,
            records: vec![TelexRecord::new(vec![
                ("path".to_owned(), "$.message".to_owned()),
                ("kind".to_owned(), "StringLiteral".to_owned()),
                ("value".to_owned(), "hello".to_owned()),
            ])],
        };
        let encoded = encode_film(&stream, &[]).expect("Film must encode");
        decode_film_view(&encoded)
            .expect("Film view must decode")
            .to_owned_unvalidated()
    };

    assert_eq!(owned.records[0].get("value"), Some("hello"));
}

fn borrows(input: &[u8], value: &[u8]) -> bool {
    let input_start = input.as_ptr() as usize;
    let input_end = input_start.saturating_add(input.len());
    let value_start = value.as_ptr() as usize;
    let value_end = value_start.saturating_add(value.len());
    input_start <= value_start && value_end <= input_end
}

fn decode_hex(input: &str) -> Vec<u8> {
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).expect("test hex must be ASCII");
            u8::from_str_radix(pair, 16).expect("test hex must contain digits")
        })
        .collect()
}
