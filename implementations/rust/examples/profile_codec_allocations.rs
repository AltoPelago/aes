use std::env;
use std::hint::black_box;
use std::process::Command;

use aes_telex::film::{
    FilmLimits, FilmStream, decode_film_view, decode_film_with_limits,
    decode_film_with_limits_path_arena, encode_film_with_limits,
};
use aes_telex::film_candidate_c::{
    decode_film_candidate_c_compact_with_limits,
    decode_film_candidate_c_validated_compact_path_arena_with_limits,
    decode_film_candidate_c_validated_compact_with_limits, decode_film_candidate_c_with_limits,
    encode_film_candidate_c_with_limits, index_film_candidate_c,
};
use aes_telex::{
    TelexLimits, encode_telex_with_projection_and_limits, parse_telex_with_limits,
    validate_telex_records_with_projection_and_limits,
};

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

const EVENT_COUNT: usize = 100_000;
const OPERATIONS: &[&str] = &[
    "aes-validate-resident",
    "film-decode-syntax",
    "film-decode-owned",
    "film-decode-complete",
    "film-decode-complete-path-arena",
    "film-encode",
    "film-c-index",
    "film-c-decode-compact",
    "film-c-decode-compact-complete",
    "film-c-decode-compact-complete-path-arena",
    "film-c-decode-complete",
    "film-c-encode",
    "telex-decode-syntax",
    "telex-decode-complete",
    "telex-encode",
];

struct OperationInputs<'a> {
    telex: &'a str,
    film: &'a [u8],
    film_c: &'a [u8],
    parsed: &'a aes_telex::ParsedTelex,
    stream: &'a FilmStream,
    film_limits: &'a FilmLimits,
    limits: &'a TelexLimits,
}

fn main() {
    let arguments = env::args().collect::<Vec<_>>();
    if arguments.get(1).map(String::as_str) == Some("--child") {
        let operation = arguments
            .get(2)
            .expect("allocation profiler child requires an operation");
        let events = arguments
            .get(3)
            .expect("allocation profiler child requires an event count")
            .parse::<usize>()
            .expect("allocation profiler event count must be an integer");
        run_child(operation, events);
        return;
    }

    println!("# profiler=dhat-0.3.3");
    println!("# implementation=rust");
    println!("# os={}", env::consts::OS);
    println!("# architecture={}", env::consts::ARCH);
    println!(
        "operation,events,total_allocations,total_bytes,peak_live_allocations,peak_live_bytes,end_live_allocations,end_live_bytes"
    );
    let executable = env::current_exe().expect("allocation profiler executable path must resolve");
    for operation in OPERATIONS {
        let status = Command::new(&executable)
            .args(["--child", operation, &EVENT_COUNT.to_string()])
            .status()
            .expect("allocation profiler child must launch");
        assert!(
            status.success(),
            "allocation profiler child failed: {operation}"
        );
    }
}

fn run_child(operation: &str, events: usize) {
    assert!((1..=100_000).contains(&events));
    assert!(
        OPERATIONS.contains(&operation),
        "unknown operation: {operation}"
    );

    let limits = TelexLimits {
        max_list_items: events,
        ..TelexLimits::default()
    };
    let film_limits = FilmLimits::default();
    let telex = build_telex(events);
    let parsed = parse_telex_with_limits(&telex, &limits).expect("generated Telex must parse");
    let stream = FilmStream::from(&parsed);
    let film = encode_film_with_limits(&stream, &[], &film_limits, &limits)
        .expect("generated records must encode as Film");
    let film_c = encode_film_candidate_c_with_limits(&stream, &[], &film_limits, &limits)
        .expect("generated records must encode as Candidate C");
    let inputs = OperationInputs {
        telex: &telex,
        film: &film,
        film_c: &film_c,
        parsed: &parsed,
        stream: &stream,
        film_limits: &film_limits,
        limits: &limits,
    };

    // Warm allocator, hashing, and parser code before beginning the measured
    // lifetime. DHAT deliberately cannot reset within one process, so every
    // operation runs in its own child process.
    run_operation(operation, &inputs);

    let profiler = dhat::Profiler::builder().testing().build();
    run_operation(operation, &inputs);
    let stats = dhat::HeapStats::get();
    drop(profiler);

    println!(
        "{operation},{events},{},{},{},{},{},{}",
        stats.total_blocks,
        stats.total_bytes,
        stats.max_blocks,
        stats.max_bytes,
        stats.curr_blocks,
        stats.curr_bytes,
    );
}

fn run_operation(operation: &str, inputs: &OperationInputs<'_>) {
    let OperationInputs {
        telex,
        film,
        film_c,
        parsed,
        stream,
        film_limits,
        limits,
    } = inputs;
    match operation {
        "aes-validate-resident" => {
            let result = validate_telex_records_with_projection_and_limits(
                black_box(&parsed.records),
                black_box(&parsed.profile),
                black_box(parsed.projection.as_deref()),
                &[],
                limits,
            );
            assert!(result.valid);
            black_box(result);
        }
        "film-decode-syntax" => {
            black_box(decode_film_view(black_box(film)).expect("generated Film must decode"));
        }
        "film-decode-owned" => {
            let view = decode_film_view(black_box(film)).expect("generated Film must decode");
            black_box(view.to_owned_unvalidated());
        }
        "film-decode-complete" => {
            black_box(
                decode_film_with_limits(black_box(film), &[], film_limits, limits)
                    .expect("generated Film must decode"),
            );
        }
        "film-decode-complete-path-arena" => {
            black_box(
                decode_film_with_limits_path_arena(black_box(film), &[], film_limits, limits)
                    .expect("generated Film path arena must decode"),
            );
        }
        "film-encode" => {
            black_box(
                encode_film_with_limits(black_box(stream), &[], film_limits, limits)
                    .expect("generated records must encode as Film"),
            );
        }
        "film-c-index" => {
            black_box(
                index_film_candidate_c(black_box(film_c), film_limits)
                    .expect("generated Candidate C must index"),
            );
        }
        "film-c-decode-compact" => {
            black_box(
                decode_film_candidate_c_compact_with_limits(black_box(film_c), film_limits, limits)
                    .expect("generated Candidate C must compact-decode"),
            );
        }
        "film-c-decode-compact-complete" => {
            black_box(
                decode_film_candidate_c_validated_compact_with_limits(
                    black_box(film_c),
                    &[],
                    film_limits,
                    limits,
                )
                .expect("generated Candidate C must compact-decode and validate"),
            );
        }
        "film-c-decode-compact-complete-path-arena" => {
            black_box(
                decode_film_candidate_c_validated_compact_path_arena_with_limits(
                    black_box(film_c),
                    &[],
                    film_limits,
                    limits,
                )
                .expect("generated Candidate C path arena must validate"),
            );
        }
        "film-c-decode-complete" => {
            black_box(
                decode_film_candidate_c_with_limits(black_box(film_c), &[], film_limits, limits)
                    .expect("generated Candidate C must decode and validate"),
            );
        }
        "film-c-encode" => {
            black_box(
                encode_film_candidate_c_with_limits(black_box(stream), &[], film_limits, limits)
                    .expect("generated records must encode as Candidate C"),
            );
        }
        "telex-decode-syntax" => {
            black_box(
                parse_telex_with_limits(black_box(telex), limits)
                    .expect("generated Telex must parse"),
            );
        }
        "telex-decode-complete" => {
            let decoded = parse_telex_with_limits(black_box(telex), limits)
                .expect("generated Telex must parse");
            let validation = validate_telex_records_with_projection_and_limits(
                &decoded.records,
                &decoded.profile,
                decoded.projection.as_deref(),
                &[],
                limits,
            );
            assert!(validation.valid);
            black_box((decoded, validation));
        }
        "telex-encode" => {
            black_box(
                encode_telex_with_projection_and_limits(
                    black_box(&parsed.records),
                    parsed.profile_explicit.then_some(parsed.profile.as_str()),
                    parsed
                        .projection_explicit
                        .then_some(parsed.projection.as_deref())
                        .flatten(),
                    limits,
                )
                .expect("generated records must encode as Telex"),
            );
        }
        _ => unreachable!("operation was checked before dispatch"),
    }
}

fn build_telex(event_count: usize) -> String {
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
