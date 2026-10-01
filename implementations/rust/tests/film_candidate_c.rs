use aes_telex::film::{FilmLimits, FilmStream};
use aes_telex::film_candidate_c::{
    FILM_CANDIDATE_C_CHECKPOINT_INTERVAL, FILM_CANDIDATE_C_PREAMBLE, decode_film_candidate_c,
    decode_film_candidate_c_compact, decode_film_candidate_c_compact_block_with_limits,
    decode_film_candidate_c_compact_cached_paths_with_limits,
    decode_film_candidate_c_compact_with_limits, decode_film_candidate_c_validated_compact,
    decode_film_candidate_c_validated_compact_path_arena_with_limits, encode_film_candidate_c,
    index_film_candidate_c, validate_film_candidate_c_compact,
    validate_film_candidate_c_compact_path_arena,
};
use aes_telex::{
    AEON_DOCUMENT_PROJECTION, ClarifierKind, DatatypeClarifier, DatatypeDescriptor,
    GenericArgument, PARTIAL_AES_PROFILE, TelexLimits, TelexRecord,
    validate_telex_records_with_projection_and_limits,
};

#[test]
fn checkpointed_comparator_round_trips_across_multiple_blocks() {
    let stream = hierarchical_stream(600);
    let encoded = encode_film_candidate_c(&stream, &[]).expect("Candidate C must encode");
    assert_eq!(
        encoded.get(..FILM_CANDIDATE_C_PREAMBLE.len()),
        Some(FILM_CANDIDATE_C_PREAMBLE.as_slice())
    );

    let compact =
        decode_film_candidate_c_compact(&encoded).expect("compact Candidate C must decode");
    assert_eq!(compact.records().len(), 600);
    assert_eq!(compact.cached_path_count(), 0);
    let cached = decode_film_candidate_c_compact_cached_paths_with_limits(
        &encoded,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
    .expect("cached-path Candidate C must decode");
    assert_eq!(cached.cached_path_count(), 600);
    assert!(cached.path_evidence_bytes() > 0);
    assert_eq!(compact.to_owned_unvalidated(), stream);
    assert_eq!(
        decode_film_candidate_c_validated_compact_path_arena_with_limits(
            &encoded,
            &[],
            &FilmLimits::default(),
            &TelexLimits::default(),
        )
        .expect("path-arena Candidate C must validate")
        .to_owned_unvalidated(),
        stream
    );
    assert_eq!(
        decode_film_candidate_c(&encoded, &[]).expect("Candidate C must validate"),
        stream
    );

    let tokens = address_tokens(&encoded);
    for (index, (_, token)) in tokens.into_iter().enumerate() {
        if index.is_multiple_of(FILM_CANDIDATE_C_CHECKPOINT_INTERVAL) {
            assert_eq!(token, 1, "record {index} must be absolute");
        } else {
            assert_eq!(token & 1, 0, "record {index} must be delta encoded");
        }
    }
}

#[test]
fn missing_checkpoint_is_rejected() {
    let mut encoded =
        encode_film_candidate_c(&hierarchical_stream(2), &[]).expect("Candidate C must encode");
    let (offset, token) = address_tokens(&encoded)[0];
    assert_eq!(token, 1);
    encoded[offset] = 0;
    let error = decode_film_candidate_c_compact(&encoded)
        .expect_err("record zero must carry an absolute checkpoint");
    assert_eq!(error.code, "FILM_CHECKPOINT_SCHEDULE");
    assert_eq!(error.record, Some(0));
}

#[test]
fn extra_checkpoint_is_rejected() {
    let mut encoded =
        encode_film_candidate_c(&hierarchical_stream(2), &[]).expect("Candidate C must encode");
    let (offset, token) = address_tokens(&encoded)[1];
    assert!(token < 0x7f, "test fixture uses a one-byte token");
    encoded[offset] |= 1;
    let error = decode_film_candidate_c_compact(&encoded)
        .expect_err("non-checkpoint records cannot carry the absolute marker");
    assert_eq!(error.code, "FILM_CHECKPOINT_SCHEDULE");
    assert_eq!(error.record, Some(1));
}

#[test]
fn reusable_address_buffer_still_rejects_a_shorter_equivalent_prefix() {
    let stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: ["$.a", "$.b"]
            .into_iter()
            .map(|path| {
                TelexRecord::new(vec![
                    ("path".to_owned(), path.to_owned()),
                    ("kind".to_owned(), "StringLiteral".to_owned()),
                    ("value".to_owned(), "x".to_owned()),
                ])
            })
            .collect(),
    };
    let mut encoded = encode_film_candidate_c(&stream, &[]).expect("fixture must encode");
    let first_frame = context_end(&encoded);
    let (first_length, first_payload) = read_uleb(&encoded, first_frame);
    let second_frame = first_payload + first_length;
    let (second_length, second_payload) = read_uleb(&encoded, second_frame);
    assert!(second_length < 0x7f, "test fixture uses a one-byte frame");

    let token_offset = second_payload + 2;
    let suffix_length_offset = token_offset + 1;
    let suffix_offset = suffix_length_offset + 1;
    assert_eq!(encoded[token_offset], 4, "two retained bytes, tagged");
    assert_eq!(encoded[suffix_length_offset], 1);
    encoded[second_frame] += 1;
    encoded[token_offset] = 2;
    encoded[suffix_length_offset] = 2;
    encoded.insert(suffix_offset, b'.');

    let error = decode_film_candidate_c_compact(&encoded)
        .expect_err("an equivalent address must use its longest prefix");
    assert_eq!(error.code, "FILM_COMPARATOR_NONCANONICAL");
    assert_eq!(error.component, "address-token");
    assert_eq!(error.record, Some(1));
}

#[test]
fn compact_side_tables_preserve_every_record_field() {
    let descriptor = DatatypeDescriptor {
        datatype: "map".to_owned(),
        generics: vec![
            GenericArgument::Datatype(DatatypeDescriptor {
                datatype: "list".to_owned(),
                generics: vec![GenericArgument::NumberLiteral("4".to_owned())],
                clarifiers: Vec::new(),
            }),
            GenericArgument::NumberLiteral("8".to_owned()),
        ],
        clarifiers: vec![DatatypeClarifier {
            kind: ClarifierKind::StringLiteral,
            value: ".".to_owned(),
        }],
    };
    let stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: vec![TelexRecord::with_datatype(
            vec![
                ("path".to_owned(), "$.items[0]".to_owned()),
                ("kind".to_owned(), "NumberLiteral".to_owned()),
                ("datatype".to_owned(), "map".to_owned()),
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
        )],
    };
    let encoded =
        encode_film_candidate_c(&stream, &["x.example.note"]).expect("Candidate C must encode");
    let compact =
        decode_film_candidate_c_compact(&encoded).expect("compact Candidate C must decode");
    assert_eq!(compact.to_owned_unvalidated(), stream);
    assert_eq!(
        decode_film_candidate_c(&encoded, &["x.example.note"])
            .expect("complete Candidate C must decode"),
        stream
    );
    assert_eq!(
        decode_film_candidate_c_validated_compact(&encoded, &["x.example.note"])
            .expect("native compact validation must pass")
            .to_owned_unvalidated(),
        stream
    );
}

#[test]
fn native_compact_validation_matches_portable_diagnostics() {
    let stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: vec![TelexRecord::new(vec![
            ("path".to_owned(), "$.value".to_owned()),
            ("kind".to_owned(), "StringLiteral".to_owned()),
            ("value".to_owned(), "value".to_owned()),
            ("x.example.note".to_owned(), "note".to_owned()),
        ])],
    };
    let encoded =
        encode_film_candidate_c(&stream, &["x.example.note"]).expect("extension must encode");
    let compact = decode_film_candidate_c_compact(&encoded).expect("compact decode must pass");
    let error = validate_film_candidate_c_compact(&compact, &[], &TelexLimits::default())
        .expect_err("an unregistered extension must fail native compact validation");
    let arena_error =
        validate_film_candidate_c_compact_path_arena(&compact, &[], &TelexLimits::default())
            .expect_err("the path-arena adapter must preserve extension validation");
    let expected = validate_telex_records_with_projection_and_limits(
        &stream.records,
        &stream.profile,
        stream.projection.as_deref(),
        &[],
        &TelexLimits::default(),
    );
    assert!(!expected.valid);
    assert_eq!(error.code, "FILM_AES_INVALID");
    assert_eq!(error.diagnostics, expected.diagnostics);
    assert_eq!(arena_error.diagnostics, expected.diagnostics);
}

#[test]
fn cached_paths_fall_back_to_portable_invalid_path_diagnostics() {
    let mut stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: vec![TelexRecord::new(vec![
            ("path".to_owned(), "$.value".to_owned()),
            ("kind".to_owned(), "StringLiteral".to_owned()),
            ("value".to_owned(), "value".to_owned()),
        ])],
    };
    let mut encoded = encode_film_candidate_c(&stream, &[]).expect("fixture must encode");
    let token_offset = address_tokens(&encoded)[0].0;
    let (_, suffix_length_offset) = read_uleb(&encoded, token_offset);
    let (suffix_length, suffix_start) = read_uleb(&encoded, suffix_length_offset);
    assert_eq!(suffix_length, "$.value".len());
    encoded[suffix_start] = b'x';

    stream.records[0] = TelexRecord::new(vec![
        ("path".to_owned(), "x.value".to_owned()),
        ("kind".to_owned(), "StringLiteral".to_owned()),
        ("value".to_owned(), "value".to_owned()),
    ]);
    let expected = validate_telex_records_with_projection_and_limits(
        &stream.records,
        &stream.profile,
        None,
        &[],
        &TelexLimits::default(),
    );
    assert!(!expected.valid);

    let lean = decode_film_candidate_c_compact(&encoded).expect("physical decode must pass");
    let cached = decode_film_candidate_c_compact_cached_paths_with_limits(
        &encoded,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
    .expect("cached physical decode must pass");
    assert_eq!(cached.cached_path_count(), 0);
    for decoded in [&lean, &cached] {
        let error = validate_film_candidate_c_compact(decoded, &[], &TelexLimits::default())
            .expect_err("invalid path must fail portable validation");
        assert_eq!(error.diagnostics, expected.diagnostics);
        let arena_error =
            validate_film_candidate_c_compact_path_arena(decoded, &[], &TelexLimits::default())
                .expect_err("invalid path must fail path-arena validation");
        assert_eq!(arena_error.diagnostics, expected.diagnostics);
    }
}

#[test]
fn transient_path_arena_matches_path_limit_diagnostics() {
    let encoded =
        encode_film_candidate_c(&hierarchical_stream(2), &[]).expect("Candidate C must encode");
    let compact = decode_film_candidate_c_compact(&encoded).expect("compact decode must pass");
    let limits = TelexLimits {
        max_path_depth: 1,
        ..TelexLimits::default()
    };
    let expected = validate_film_candidate_c_compact(&compact, &[], &limits)
        .expect_err("depth-two paths must exceed the test limit");
    let actual = validate_film_candidate_c_compact_path_arena(&compact, &[], &limits)
        .expect_err("the arena must enforce the same path limit");
    assert_eq!(actual.diagnostics, expected.diagnostics);
}

#[test]
fn compact_text_slab_obeys_the_decoded_payload_limit() {
    let encoded =
        encode_film_candidate_c(&hierarchical_stream(2), &[]).expect("Candidate C must encode");
    let limits = TelexLimits {
        max_decoded_payload_bytes: 8,
        ..TelexLimits::default()
    };
    let error =
        decode_film_candidate_c_compact_with_limits(&encoded, &FilmLimits::default(), &limits)
            .expect_err("the compact slab must be bounded");
    assert_eq!(error.code, "FILM_LIMIT_EXCEEDED");
}

#[test]
fn compact_decoder_bounds_retained_addresses_across_both_planes() {
    let stream = two_plane_long_address_stream();
    let encoded = encode_film_candidate_c(&stream, &[]).expect("fixture must encode");
    let limits = FilmLimits {
        max_buffered_bytes: 120,
        ..FilmLimits::default()
    };
    let error =
        decode_film_candidate_c_compact_with_limits(&encoded, &limits, &TelexLimits::default())
            .expect_err("retained body and header addresses must share one buffer limit");
    assert_eq!(error.code, "FILM_LIMIT_EXCEEDED");
    assert_eq!(error.component, "previous-addresses");
    assert_eq!(error.record, Some(1));
}

#[test]
fn retained_directory_recovers_after_an_earlier_block_is_damaged() {
    let stream = hierarchical_stream(600);
    let mut encoded = encode_film_candidate_c(&stream, &[]).expect("Candidate C must encode");
    let index = index_film_candidate_c(&encoded, &FilmLimits::default())
        .expect("Candidate C checkpoints must index");
    assert_eq!(index.checkpoints.len(), 3);

    let (_, first_token_end) = read_uleb(&encoded, address_tokens(&encoded)[0].0);
    let (_, first_suffix) = read_uleb(&encoded, first_token_end);
    encoded[first_suffix] = 0xff;
    assert!(decode_film_candidate_c_compact(&encoded).is_err());

    let recovered = decode_film_candidate_c_compact_block_with_limits(
        &encoded,
        &index,
        1,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
    .expect("the next absolute block must decode independently");
    assert_eq!(
        recovered.to_owned_unvalidated().records,
        stream.records[256..512]
    );
}

#[test]
fn checkpoint_blocks_reset_header_and_body_planes_independently() {
    let mut records = vec![TelexRecord::new(vec![
        ("header".to_owned(), "$.[\"aeon:conventions\"]".to_owned()),
        ("kind".to_owned(), "ListNode".to_owned()),
    ])];
    records.extend((0..299).map(|index| {
        TelexRecord::new(vec![
            (
                "header".to_owned(),
                format!("$.[\"aeon:conventions\"][{index}]"),
            ),
            ("kind".to_owned(), "StringLiteral".to_owned()),
            ("value".to_owned(), format!("convention-{index}")),
        ])
    }));
    records.push(TelexRecord::new(vec![
        ("path".to_owned(), "$.value".to_owned()),
        ("kind".to_owned(), "StringLiteral".to_owned()),
        ("value".to_owned(), "body".to_owned()),
    ]));
    let stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: Some(AEON_DOCUMENT_PROJECTION.to_owned()),
        projection_explicit: true,
        records,
    };
    let encoded = encode_film_candidate_c(&stream, &[])
        .expect("projected Candidate C must encode both address planes");
    let tokens = address_tokens(&encoded);
    assert_eq!(tokens[256].1, 1, "the block's header plane must reset");
    assert_eq!(tokens[300].1, 1, "the block's body plane must reset");

    let index = index_film_candidate_c(&encoded, &FilmLimits::default())
        .expect("projected checkpoints must index");
    assert_eq!(index.checkpoints.len(), 2);
    let block = decode_film_candidate_c_compact_block_with_limits(
        &encoded,
        &index,
        1,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
    .expect("both planes in the checkpoint block must decode independently");
    assert_eq!(block.to_owned_unvalidated().records, stream.records[256..]);
}

fn hierarchical_stream(count: usize) -> FilmStream {
    FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: (0..count)
            .map(|index| {
                TelexRecord::new(vec![
                    ("path".to_owned(), format!("$.items[{index}]")),
                    ("kind".to_owned(), "StringLiteral".to_owned()),
                    ("value".to_owned(), format!("value-{index}")),
                ])
            })
            .collect(),
    }
}

fn two_plane_long_address_stream() -> FilmStream {
    FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: Some(AEON_DOCUMENT_PROJECTION.to_owned()),
        projection_explicit: true,
        records: vec![
            TelexRecord::new(vec![
                (
                    "header".to_owned(),
                    format!("$.[\"aeon:{}\"]", "h".repeat(72)),
                ),
                ("kind".to_owned(), "StringLiteral".to_owned()),
                ("value".to_owned(), "header".to_owned()),
            ]),
            TelexRecord::new(vec![
                ("path".to_owned(), format!("$.{}", "b".repeat(80))),
                ("kind".to_owned(), "StringLiteral".to_owned()),
                ("value".to_owned(), "body".to_owned()),
            ]),
        ],
    }
}

fn address_tokens(bytes: &[u8]) -> Vec<(usize, u64)> {
    let mut position = context_end(bytes);
    let mut tokens = Vec::new();
    while position < bytes.len() {
        let (payload_length, payload_offset) = read_uleb(bytes, position);
        let token_offset = payload_offset + 2;
        let (token, _) = read_uleb(bytes, token_offset);
        tokens.push((token_offset, token as u64));
        position = payload_offset + payload_length;
    }
    tokens
}

fn context_end(bytes: &[u8]) -> usize {
    let mut position = FILM_CANDIDATE_C_PREAMBLE.len();
    let context = bytes[position];
    position += 1;
    if context & 0x01 != 0 {
        let (length, payload) = read_uleb(bytes, position);
        position = payload + length;
    }
    if context & 0x02 != 0 {
        let (length, payload) = read_uleb(bytes, position);
        position = payload + length;
    }
    position
}

fn read_uleb(bytes: &[u8], mut position: usize) -> (usize, usize) {
    let mut value = 0_usize;
    let mut shift = 0_u32;
    loop {
        let byte = bytes[position];
        position += 1;
        value |= usize::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return (value, position);
        }
        shift += 7;
    }
}
