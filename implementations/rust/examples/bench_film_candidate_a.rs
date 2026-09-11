use std::collections::HashSet;
use std::env;
use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::time::{Duration, Instant};

use aes_telex::film_candidate_a::{FilmStream, decode_film_candidate_a, encode_film_candidate_a};
use aes_telex::film_candidate_b::{decode_film_candidate_b, encode_film_candidate_b};
use aes_telex::{encode_telex_with_projection, parse_telex};

struct Case {
    name: &'static str,
    events: usize,
    iterations: usize,
    warmups: usize,
}

fn main() {
    let cases = [
        Case {
            name: "small",
            events: 100,
            iterations: 1_000,
            warmups: 20,
        },
        Case {
            name: "medium",
            events: 10_000,
            iterations: 30,
            warmups: 3,
        },
        Case {
            name: "default-list-limit",
            events: 65_537,
            iterations: 5,
            warmups: 1,
        },
    ];

    println!(
        "case,events,telex_bytes,candidate_a_bytes,candidate_b_bytes,a_over_telex,b_over_a,a_encode_ms,b_encode_ms,a_decode_ms,b_decode_ms,a_to_telex_ms,address_bytes,adjacent_address_prefix_bytes,datatype_bytes,repeated_datatype_bytes,value_bytes,repeated_value_bytes"
    );
    for case in cases {
        let telex = build_telex(case.events);
        let parsed = parse_telex(&telex).expect("generated Telex must parse");
        let stream = FilmStream::from(&parsed);
        let repetition = repetition_measurements(&stream);
        let candidate_a = encode_film_candidate_a(&stream, &[]).expect("Candidate A must encode");
        let candidate_b = encode_film_candidate_b(&stream, &[]).expect("Candidate B must encode");
        write_samples(case.name, &telex, &candidate_a, &candidate_b);
        let a_encode = measure(case.iterations, case.warmups, || {
            black_box(
                encode_film_candidate_a(black_box(&stream), &[]).expect("Candidate A must encode"),
            );
        });
        let b_encode = measure(case.iterations, case.warmups, || {
            black_box(
                encode_film_candidate_b(black_box(&stream), &[]).expect("Candidate B must encode"),
            );
        });
        let a_decode = measure(case.iterations, case.warmups, || {
            black_box(
                decode_film_candidate_a(black_box(&candidate_a), &[])
                    .expect("Candidate A must decode"),
            );
        });
        let b_decode = measure(case.iterations, case.warmups, || {
            black_box(
                decode_film_candidate_b(black_box(&candidate_b), &[])
                    .expect("Candidate B must decode"),
            );
        });
        let a_to_telex = measure(case.iterations, case.warmups, || {
            let decoded = decode_film_candidate_a(black_box(&candidate_a), &[])
                .expect("Candidate A must decode");
            black_box(
                encode_telex_with_projection(&decoded.records, None, None)
                    .expect("decoded records must encode as Telex"),
            );
        });
        println!(
            "{},{},{},{},{},{:.4},{:.4},{:.3},{:.3},{:.3},{:.3},{:.3},{},{},{},{},{},{}",
            case.name,
            case.events,
            telex.len(),
            candidate_a.len(),
            candidate_b.len(),
            candidate_a.len() as f64 / telex.len() as f64,
            candidate_b.len() as f64 / candidate_a.len() as f64,
            milliseconds(a_encode),
            milliseconds(b_encode),
            milliseconds(a_decode),
            milliseconds(b_decode),
            milliseconds(a_to_telex),
            repetition.address_bytes,
            repetition.adjacent_address_prefix_bytes,
            repetition.datatype_bytes,
            repetition.repeated_datatype_bytes,
            repetition.value_bytes,
            repetition.repeated_value_bytes,
        );
    }
}

fn write_samples(name: &str, telex: &str, candidate_a: &[u8], candidate_b: &[u8]) {
    let Ok(directory) = env::var("FILM_BENCH_OUTPUT_DIR") else {
        return;
    };
    let directory = Path::new(&directory);
    fs::create_dir_all(directory).expect("benchmark output directory must be creatable");
    fs::write(directory.join(format!("{name}.telex.aes")), telex)
        .expect("Telex sample must be writable");
    fs::write(
        directory.join(format!("{name}.candidate-a.bin")),
        candidate_a,
    )
    .expect("Candidate A sample must be writable");
    fs::write(
        directory.join(format!("{name}.candidate-b.bin")),
        candidate_b,
    )
    .expect("Candidate B sample must be writable");
}

#[derive(Default)]
struct RepetitionMeasurements {
    address_bytes: usize,
    adjacent_address_prefix_bytes: usize,
    datatype_bytes: usize,
    repeated_datatype_bytes: usize,
    value_bytes: usize,
    repeated_value_bytes: usize,
}

fn repetition_measurements(stream: &FilmStream) -> RepetitionMeasurements {
    let mut result = RepetitionMeasurements::default();
    let mut previous_address: Option<&str> = None;
    let mut datatypes = HashSet::new();
    let mut values = HashSet::new();
    for record in &stream.records {
        if let Some(address) = record.get("path").or_else(|| record.get("header")) {
            result.address_bytes = result.address_bytes.saturating_add(address.len());
            if let Some(previous) = previous_address {
                result.adjacent_address_prefix_bytes = result
                    .adjacent_address_prefix_bytes
                    .saturating_add(common_utf8_prefix_bytes(previous, address));
            }
            previous_address = Some(address);
        }
        if let Some(datatype) = record.get("datatype") {
            result.datatype_bytes = result.datatype_bytes.saturating_add(datatype.len());
            if !datatypes.insert(datatype) {
                result.repeated_datatype_bytes = result
                    .repeated_datatype_bytes
                    .saturating_add(datatype.len());
            }
        }
        if let Some(value) = record.get("value") {
            result.value_bytes = result.value_bytes.saturating_add(value.len());
            if !values.insert(value) {
                result.repeated_value_bytes =
                    result.repeated_value_bytes.saturating_add(value.len());
            }
        }
    }
    result
}

fn common_utf8_prefix_bytes(left: &str, right: &str) -> usize {
    let mut length = left
        .as_bytes()
        .iter()
        .zip(right.as_bytes())
        .take_while(|(left, right)| left == right)
        .count();
    while !left.is_char_boundary(length) || !right.is_char_boundary(length) {
        length = length.saturating_sub(1);
    }
    length
}

fn measure(iterations: usize, warmups: usize, mut operation: impl FnMut()) -> Duration {
    for _ in 0..warmups {
        operation();
    }
    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        operation();
        samples.push(start.elapsed());
    }
    samples.sort_unstable();
    samples[iterations / 2]
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn build_telex(event_count: usize) -> String {
    assert!((1..=65_537).contains(&event_count));
    let mut output = String::with_capacity(event_count * 60);
    output.push_str("telex.aes=1\n\npath=$.items\nkind=ListNode\ndatatype=list<string>\n");
    for index in 0..event_count - 1 {
        output.push_str("\npath=$.items[");
        output.push_str(&index.to_string());
        output.push_str("]\n");
        match index % 4 {
            0 => {
                output.push_str("kind=StringLiteral\nvalue=value-");
                output.push_str(&index.to_string());
                output.push_str("-café\n");
            }
            1 => {
                output.push_str("kind=NumberLiteral\ndatatype=int\nvalue=");
                output.push_str(&index.to_string());
                output.push('\n');
            }
            2 => {
                output.push_str("kind=BooleanLiteral\nvalue=");
                output.push_str(if index % 8 == 2 { "true\n" } else { "false\n" });
            }
            _ => output.push_str("kind=CloneReference\nvalue=$.items[0]\n"),
        }
    }
    output
}
