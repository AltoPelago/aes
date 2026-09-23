#![no_main]

use aes_telex::film::{decode_film, decode_film_view, decode_film_with_limits_path_arena};
use aes_telex::film_candidate_b::decode_film_candidate_b;
use aes_telex::film_candidate_c::{
    decode_film_candidate_c_compact, decode_film_candidate_c_validated_compact,
    decode_film_candidate_c_validated_compact_cached_paths_with_limits,
    decode_film_candidate_c_validated_compact_path_arena_with_limits, encode_film_candidate_c,
};
use aes_telex::{TelexLimits, film::FilmLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let _ = decode_film_view(input);
    let _ = decode_film_with_limits_path_arena(
        input,
        &[],
        &FilmLimits::default(),
        &TelexLimits::default(),
    );
    if let Ok(stream) = decode_film(input, &[]) {
        if let Ok(candidate_c) = encode_film_candidate_c(&stream, &[]) {
            let _ = decode_film_candidate_c_compact(&candidate_c);
            let _ = decode_film_candidate_c_validated_compact(&candidate_c, &[]);
            let _ = decode_film_candidate_c_validated_compact_cached_paths_with_limits(
                &candidate_c,
                &[],
                &FilmLimits::default(),
                &TelexLimits::default(),
            );
            let _ = decode_film_candidate_c_validated_compact_path_arena_with_limits(
                &candidate_c,
                &[],
                &FilmLimits::default(),
                &TelexLimits::default(),
            );
        }
    }
    let _ = decode_film_candidate_b(input, &[]);
    let _ = decode_film_candidate_c_compact(input);
    let _ = decode_film_candidate_c_validated_compact(input, &[]);
    let _ = decode_film_candidate_c_validated_compact_cached_paths_with_limits(
        input,
        &[],
        &FilmLimits::default(),
        &TelexLimits::default(),
    );
    let _ = decode_film_candidate_c_validated_compact_path_arena_with_limits(
        input,
        &[],
        &FilmLimits::default(),
        &TelexLimits::default(),
    );
});
