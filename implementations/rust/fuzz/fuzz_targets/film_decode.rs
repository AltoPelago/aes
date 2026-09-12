#![no_main]

use aes_telex::film::{decode_film, decode_film_view};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let _ = decode_film_view(input);
    let _ = decode_film(input, &[]);
});
