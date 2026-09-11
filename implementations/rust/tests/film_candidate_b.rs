use aes_telex::film_candidate_a::{FILM_V1_PREAMBLE, FilmStream, decode_film_candidate_a};
use aes_telex::film_candidate_b::{
    FILM_CANDIDATE_B_PREAMBLE, decode_film_candidate_b, encode_film_candidate_b,
};
use aes_telex::{PARTIAL_AES_PROFILE, TelexRecord};

#[test]
fn comparator_has_distinct_identity_and_round_trips() {
    let stream = hierarchical_stream();
    let candidate_a = aes_telex::film_candidate_a::encode_film_candidate_a(&stream, &[])
        .expect("Candidate A must encode");
    let candidate_b = encode_film_candidate_b(&stream, &[]).expect("Candidate B must encode");
    assert_eq!(
        candidate_a.get(..FILM_V1_PREAMBLE.len()),
        Some(FILM_V1_PREAMBLE.as_slice())
    );
    assert_eq!(
        candidate_b.get(..FILM_CANDIDATE_B_PREAMBLE.len()),
        Some(FILM_CANDIDATE_B_PREAMBLE.as_slice())
    );
    assert_ne!(FILM_CANDIDATE_B_PREAMBLE, FILM_V1_PREAMBLE);
    assert!(candidate_b.len() < candidate_a.len());
    assert_eq!(
        decode_film_candidate_b(&candidate_b, &[]).expect("Candidate B must decode"),
        stream
    );
    assert_eq!(
        decode_film_candidate_a(&candidate_b, &[])
            .expect_err("Candidate A must reject Candidate B")
            .code,
        "FILM_INVALID_PREAMBLE"
    );
}

#[test]
fn first_record_cannot_claim_an_unavailable_prefix() {
    let mut encoded =
        encode_film_candidate_b(&hierarchical_stream(), &[]).expect("Candidate B must encode");
    let context_end = context_end(&encoded);
    let (_, payload_offset) = read_uleb(&encoded, context_end);
    let prefix_offset = payload_offset.saturating_add(2);
    encoded[prefix_offset] = 1;
    let error = decode_film_candidate_b(&encoded, &[])
        .expect_err("The first body record has no previous address");
    assert_eq!(error.code, "FILM_COMPARATOR_INVALID_PREFIX");
    assert_eq!(error.record, Some(0));
}

#[test]
fn removing_a_predecessor_breaks_stateful_record_decoding() {
    let encoded =
        encode_film_candidate_b(&hierarchical_stream(), &[]).expect("Candidate B must encode");
    let context_end = context_end(&encoded);
    let (first_length, first_payload) = read_uleb(&encoded, context_end);
    let second_frame = first_payload.saturating_add(first_length);
    let mut without_first = encoded[..context_end].to_vec();
    without_first.extend_from_slice(&encoded[second_frame..]);
    let error = decode_film_candidate_b(&without_first, &[])
        .expect_err("A stateful suffix must not survive removal of its predecessor");
    assert_eq!(error.code, "FILM_COMPARATOR_INVALID_PREFIX");
}

#[test]
fn removing_a_middle_record_can_silently_retarget_a_later_address() {
    let stream = FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: ["$.a.other", "$.a.long.x", "$.a.long.y"]
            .into_iter()
            .enumerate()
            .map(|(index, path)| {
                TelexRecord::new(vec![
                    ("path".to_owned(), path.to_owned()),
                    ("kind".to_owned(), "StringLiteral".to_owned()),
                    ("value".to_owned(), format!("value-{index}")),
                ])
            })
            .collect(),
    };
    let encoded = encode_film_candidate_b(&stream, &[]).expect("Candidate B must encode");
    let context_end = context_end(&encoded);
    let (first_length, first_payload) = read_uleb(&encoded, context_end);
    let second_frame = first_payload.saturating_add(first_length);
    let (second_length, second_payload) = read_uleb(&encoded, second_frame);
    let third_frame = second_payload.saturating_add(second_length);
    let mut without_middle = encoded[..second_frame].to_vec();
    without_middle.extend_from_slice(&encoded[third_frame..]);

    let decoded = decode_film_candidate_b(&without_middle, &[])
        .expect("The retargeted partial stream remains locally valid");
    assert_eq!(decoded.records.len(), 2);
    assert_eq!(decoded.records[0].get("path"), Some("$.a.other"));
    assert_eq!(decoded.records[1].get("path"), Some("$.a.othery"));
    assert_ne!(decoded.records[1].get("path"), Some("$.a.long.y"));
}

#[test]
fn comparator_bytes_are_canonical_for_the_same_logical_stream() {
    let stream = hierarchical_stream();
    let encoded = encode_film_candidate_b(&stream, &[]).expect("Candidate B must encode");
    let decoded = decode_film_candidate_b(&encoded, &[]).expect("Candidate B must decode");
    assert_eq!(
        encode_film_candidate_b(&decoded, &[]).expect("Candidate B must re-encode"),
        encoded
    );
}

fn hierarchical_stream() -> FilmStream {
    FilmStream {
        profile: PARTIAL_AES_PROFILE.to_owned(),
        profile_explicit: true,
        projection: None,
        projection_explicit: false,
        records: (0..32)
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

fn context_end(bytes: &[u8]) -> usize {
    let mut position = FILM_CANDIDATE_B_PREAMBLE.len();
    let control = bytes[position];
    position = position.saturating_add(1);
    if control & 0x01 != 0 {
        let (length, payload) = read_uleb(bytes, position);
        position = payload.saturating_add(length);
    }
    if control & 0x02 != 0 {
        let (length, payload) = read_uleb(bytes, position);
        position = payload.saturating_add(length);
    }
    position
}

fn read_uleb(bytes: &[u8], mut position: usize) -> (usize, usize) {
    let mut value = 0_usize;
    let mut shift = 0_u32;
    loop {
        let byte = bytes[position];
        position = position.saturating_add(1);
        value |= usize::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return (value, position);
        }
        shift = shift.saturating_add(7);
    }
}
