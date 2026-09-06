use std::hint::black_box;
use std::time::{Duration, Instant};

use aes_telex::{
    canonicalize_telex, check_telex_completeness, encode_telex_with_projection, parse_telex,
    validate_telex,
};

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
            name: "limit-scale",
            events: 100_000,
            iterations: 5,
            warmups: 1,
        },
    ];

    println!(
        "case,events,bytes,fnv1a32,parse_ms,encode_ms,validate_ms,canonicalize_ms,completeness_ms"
    );
    for case in cases {
        let telex = build_telex(case.events);
        let parse = measure(case.iterations, case.warmups, || {
            black_box(parse_telex(black_box(&telex)).expect("generated Telex must parse"));
        });
        let parsed = parse_telex(&telex).expect("generated Telex must parse");
        let encode = measure(case.iterations, case.warmups, || {
            black_box(
                encode_telex_with_projection(
                    black_box(&parsed.records),
                    parsed.profile_explicit.then_some(parsed.profile.as_str()),
                    parsed
                        .projection_explicit
                        .then_some(parsed.projection.as_deref())
                        .flatten(),
                )
                .expect("generated records must encode"),
            );
        });
        drop(parsed);
        let validate = measure(case.iterations, case.warmups, || {
            black_box(
                validate_telex(black_box(&telex), &[]).expect("generated Telex must validate"),
            );
        });
        let canonicalize = measure(case.iterations, case.warmups, || {
            black_box(
                canonicalize_telex(black_box(&telex)).expect("generated Telex must canonicalize"),
            );
        });
        let completeness = measure(case.iterations, case.warmups, || {
            black_box(
                check_telex_completeness(black_box(&telex))
                    .expect("generated Telex must support completeness checks"),
            );
        });
        println!(
            "{},{},{},{:08x},{:.3},{:.3},{:.3},{:.3},{:.3}",
            case.name,
            case.events,
            telex.len(),
            fnv1a32(telex.as_bytes()),
            milliseconds(parse),
            milliseconds(encode),
            milliseconds(validate),
            milliseconds(canonicalize),
            milliseconds(completeness),
        );
    }
}

fn fnv1a32(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |hash, byte| {
        (hash ^ u32::from(*byte)).wrapping_mul(0x0100_0193)
    })
}

fn measure(mut iterations: usize, warmups: usize, mut operation: impl FnMut()) -> Duration {
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
    iterations /= 2;
    samples[iterations]
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn build_telex(event_count: usize) -> String {
    assert!((1..=100_000).contains(&event_count));
    let mut output = String::with_capacity(event_count * 60);
    output.push_str("telex.aes=0\n\npath=$.items\nkind=ListNode\ndatatype=list<string>\n");
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
