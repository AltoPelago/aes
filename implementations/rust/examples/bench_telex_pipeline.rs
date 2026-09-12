use std::env;
use std::sync::mpsc::sync_channel;
use std::thread;
use std::time::{Duration, Instant};

use aes_telex::{
    TelexLimits, parse_telex_with_limits, validate_telex_records_with_projection_and_limits,
};

const EVENTS_PER_DOCUMENT: usize = 10_000;
const DOCUMENTS: usize = 32;
const ITERATIONS: usize = 7;
const WARMUPS: usize = 1;

#[derive(Clone, Copy)]
struct Timing {
    median: Duration,
    p95: Duration,
}

fn main() {
    let limits = TelexLimits {
        max_list_items: EVENTS_PER_DOCUMENT,
        ..TelexLimits::default()
    };
    let telex = build_telex(EVENTS_PER_DOCUMENT);

    println!("# benchmark=telex-document-pipeline-v1");
    println!("# topology=T0-sequential-T1-document-handoff-and-T2-document-tile");
    println!("# implementation=rust");
    println!("# os={}", env::consts::OS);
    println!("# architecture={}", env::consts::ARCH);
    println!("# documents={DOCUMENTS}");
    println!("# events-per-document={EVENTS_PER_DOCUMENT}");
    println!("# handoff=std-sync-channel;owned-provisional-parsed-telex");
    println!("# completion=consumer-validates-each-complete-document-before-counting");
    println!(
        "topology,lanes,capacity,documents,median_ms,p95_ms,documents_per_second,events_per_second,speedup"
    );

    let sequential = measure(DOCUMENTS, || run_sequential(&telex, DOCUMENTS, &limits));
    emit("T0", 1, 0, DOCUMENTS, sequential, sequential);
    for capacity in [1, 2, 4, 8] {
        let pipelined = measure(DOCUMENTS, || {
            run_document_pipeline(&telex, DOCUMENTS, capacity, &limits)
        });
        emit("T1-document", 1, capacity, DOCUMENTS, pipelined, sequential);
    }
    for capacity in [1, 2] {
        let tiled = measure(DOCUMENTS * 2, || {
            run_two_lane_tile(&telex, DOCUMENTS, capacity, &limits)
        });
        emit("T2-document", 2, capacity, DOCUMENTS * 2, tiled, sequential);
    }
}

fn run_sequential(input: &str, documents: usize, limits: &TelexLimits) -> usize {
    let mut accepted_events = 0;
    for _ in 0..documents {
        let parsed = parse_telex_with_limits(input, limits).expect("generated Telex must parse");
        accepted_events += accept_complete_document(&parsed, limits);
    }
    accepted_events
}

fn run_document_pipeline(
    input: &str,
    documents: usize,
    capacity: usize,
    limits: &TelexLimits,
) -> usize {
    let (sender, receiver) = sync_channel(capacity);
    thread::scope(|scope| {
        let producer = scope.spawn(move || {
            for _ in 0..documents {
                let parsed =
                    parse_telex_with_limits(input, limits).expect("generated Telex must parse");
                sender
                    .send(parsed)
                    .expect("semantic pipeline consumer must remain available");
            }
        });
        let consumer = scope.spawn(move || {
            let mut accepted_events = 0;
            for parsed in receiver {
                accepted_events += accept_complete_document(&parsed, limits);
            }
            accepted_events
        });

        producer
            .join()
            .expect("physical pipeline worker must finish");
        consumer
            .join()
            .expect("semantic pipeline worker must finish")
    })
}

fn run_two_lane_tile(
    input: &str,
    documents_per_lane: usize,
    capacity: usize,
    limits: &TelexLimits,
) -> usize {
    let (sender_a, receiver_a) = sync_channel(capacity);
    let (sender_b, receiver_b) = sync_channel(capacity);
    thread::scope(|scope| {
        let producer_a = scope.spawn(move || {
            produce_documents(input, documents_per_lane, limits, sender_a);
        });
        let consumer_a = scope.spawn(move || consume_documents(receiver_a, limits));
        let producer_b = scope.spawn(move || {
            produce_documents(input, documents_per_lane, limits, sender_b);
        });
        let consumer_b = scope.spawn(move || consume_documents(receiver_b, limits));

        producer_a.join().expect("lane A parser must finish");
        producer_b.join().expect("lane B parser must finish");
        consumer_a.join().expect("lane A validator must finish")
            + consumer_b.join().expect("lane B validator must finish")
    })
}

fn produce_documents(
    input: &str,
    documents: usize,
    limits: &TelexLimits,
    sender: std::sync::mpsc::SyncSender<aes_telex::ParsedTelex>,
) {
    for _ in 0..documents {
        let parsed = parse_telex_with_limits(input, limits).expect("generated Telex must parse");
        sender
            .send(parsed)
            .expect("semantic pipeline consumer must remain available");
    }
}

fn consume_documents(
    receiver: std::sync::mpsc::Receiver<aes_telex::ParsedTelex>,
    limits: &TelexLimits,
) -> usize {
    let mut accepted_events = 0;
    for parsed in receiver {
        accepted_events += accept_complete_document(&parsed, limits);
    }
    accepted_events
}

fn accept_complete_document(parsed: &aes_telex::ParsedTelex, limits: &TelexLimits) -> usize {
    let validation = validate_telex_records_with_projection_and_limits(
        &parsed.records,
        &parsed.profile,
        parsed.projection.as_deref(),
        &[],
        limits,
    );
    assert!(validation.valid, "generated Telex must be complete");
    parsed.records.len()
}

fn measure(documents: usize, mut operation: impl FnMut() -> usize) -> Timing {
    let expected_events = documents * EVENTS_PER_DOCUMENT;
    for _ in 0..WARMUPS {
        assert_eq!(
            operation(),
            expected_events,
            "every generated event must be accepted",
        );
    }
    let mut samples = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        assert_eq!(
            operation(),
            expected_events,
            "every generated event must be accepted",
        );
        samples.push(start.elapsed());
    }
    samples.sort_unstable();
    Timing {
        median: samples[samples.len() / 2],
        p95: samples[(samples.len() * 95).div_ceil(100).saturating_sub(1)],
    }
}

fn emit(
    topology: &str,
    lanes: usize,
    capacity: usize,
    documents: usize,
    timing: Timing,
    sequential: Timing,
) {
    let seconds = timing.median.as_secs_f64();
    let documents_per_second = documents as f64 / seconds;
    let sequential_documents_per_second = DOCUMENTS as f64 / sequential.median.as_secs_f64();
    println!(
        "{topology},{lanes},{capacity},{documents},{:.3},{:.3},{:.2},{:.0},{:.3}",
        milliseconds(timing.median),
        milliseconds(timing.p95),
        documents_per_second,
        documents_per_second * EVENTS_PER_DOCUMENT as f64,
        documents_per_second / sequential_documents_per_second,
    );
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
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
