use std::env;
use std::hint::black_box;
use std::time::{Duration, Instant};

use aes_telex::film::{
    FilmLimits, FilmStream, decode_film_view, decode_film_with_limits, encode_film_with_limits,
};
use aes_telex::{
    ClarifierKind, DatatypeClarifier, DatatypeDescriptor, GenericArgument, TelexLimits,
    TelexRecord, encode_telex_with_projection_and_limits, parse_telex_with_limits,
    validate_telex_records_with_projection_and_limits,
};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const COMPLETE_PROFILE: &str = "aes.complete.v1";

struct Case {
    name: &'static str,
    events: usize,
    iterations: usize,
    warmups: usize,
    payload: Payload,
}

#[derive(Clone, Copy)]
enum Payload {
    MixedShort,
    LargeAscii,
    EscapeHeavy,
    LargeUtf8,
}

struct Timing {
    median: Duration,
    p95: Duration,
    samples: Vec<Duration>,
}

struct Row<'a> {
    case: &'a str,
    encoding: &'a str,
    operation: &'a str,
    boundary: &'a str,
    size_basis: &'a str,
    events: usize,
    bytes: usize,
    iterations: usize,
    timing: Timing,
    digest: &'a str,
}

fn main() {
    let cases = [
        Case {
            name: "small",
            events: 100,
            iterations: 1_000,
            warmups: 20,
            payload: Payload::MixedShort,
        },
        Case {
            name: "medium",
            events: 10_000,
            iterations: 30,
            warmups: 3,
            payload: Payload::MixedShort,
        },
        Case {
            name: "limit-scale",
            events: 100_000,
            iterations: 5,
            warmups: 1,
            payload: Payload::MixedShort,
        },
        Case {
            name: "large-ascii",
            events: 10_000,
            iterations: 5,
            warmups: 1,
            payload: Payload::LargeAscii,
        },
        Case {
            name: "escape-heavy",
            events: 10_000,
            iterations: 5,
            warmups: 1,
            payload: Payload::EscapeHeavy,
        },
        Case {
            name: "large-utf8",
            events: 10_000,
            iterations: 5,
            warmups: 1,
            payload: Payload::LargeUtf8,
        },
    ];

    println!("# benchmark=aes-scalar-codec-phases-v1");
    println!("# topology=T0-sequential");
    println!("# implementation=rust");
    println!("# os={}", env::consts::OS);
    println!("# architecture={}", env::consts::ARCH);
    println!("# timing=median-and-p95-wall-clock;warmups-excluded");
    println!("# json-engine=serde_json;adapter=portable-aes-object-array");
    println!("# scanner-corpora=large-ascii;escape-heavy;large-utf8");
    println!(
        "case,encoding,operation,boundary,size_basis,events,bytes,iterations,median_ms,p95_ms,mb_per_second,events_per_second,sha256"
    );

    for case in &cases {
        run_case(case);
    }
}

fn run_case(case: &Case) {
    let limits = workload_limits(case.events);
    let film_limits = FilmLimits::default();
    let telex = build_telex(case.events, case.payload);
    let parsed = parse_telex_with_limits(&telex, &limits).expect("generated Telex must parse");
    assert_eq!(parsed.records.len(), case.events);

    let stream = FilmStream::from(&parsed);
    let film = encode_film_with_limits(&stream, &[], &film_limits, &limits)
        .expect("generated records must encode as Film");
    let json_value = records_to_json(&parsed.records);
    let json = serde_json::to_vec(&json_value).expect("generated records must encode as JSON");

    let telex_digest = sha256_hex(telex.as_bytes());
    let film_digest = sha256_hex(&film);
    let json_digest = sha256_hex(&json);

    let decoded_json = json_to_records(
        serde_json::from_slice(&json).expect("generated JSON must parse as a value"),
    );
    assert_eq!(decoded_json, parsed.records);
    assert_eq!(
        decode_film_with_limits(&film, &[], &film_limits, &limits)
            .expect("generated Film must decode")
            .records,
        parsed.records
    );
    assert!(
        validate_telex_records_with_projection_and_limits(
            &parsed.records,
            &parsed.profile,
            parsed.projection.as_deref(),
            &[],
            &limits,
        )
        .valid
    );

    emit(Row {
        case: case.name,
        encoding: "aes",
        operation: "validate-resident",
        boundary: "complete-aes-validity",
        size_basis: "json-equivalent-resident",
        events: case.events,
        bytes: json.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            let result = validate_telex_records_with_projection_and_limits(
                black_box(&parsed.records),
                black_box(&parsed.profile),
                black_box(parsed.projection.as_deref()),
                &[],
                &limits,
            );
            assert!(result.valid);
            black_box(result);
        }),
        digest: &json_digest,
    });

    emit(Row {
        case: case.name,
        encoding: "film",
        operation: "decode-syntax",
        boundary: "borrowed-provisional-film-view",
        size_basis: "wire-input",
        events: case.events,
        bytes: film.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            black_box(decode_film_view(black_box(&film)).expect("generated Film must decode"));
        }),
        digest: &film_digest,
    });
    emit(Row {
        case: case.name,
        encoding: "film",
        operation: "decode-owned",
        boundary: "owned-provisional-aes-records",
        size_basis: "wire-input",
        events: case.events,
        bytes: film.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            let view = decode_film_view(black_box(&film)).expect("generated Film must decode");
            black_box(view.to_owned_unvalidated());
        }),
        digest: &film_digest,
    });
    emit(Row {
        case: case.name,
        encoding: "film",
        operation: "decode-complete",
        boundary: "owned-complete-aes-stream",
        size_basis: "wire-input",
        events: case.events,
        bytes: film.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            black_box(
                decode_film_with_limits(black_box(&film), &[], &film_limits, &limits)
                    .expect("generated Film must decode"),
            );
        }),
        digest: &film_digest,
    });
    emit(Row {
        case: case.name,
        encoding: "film",
        operation: "encode",
        boundary: "resident-records-to-validated-canonical-bytes",
        size_basis: "wire-output",
        events: case.events,
        bytes: film.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            black_box(
                encode_film_with_limits(black_box(&stream), &[], &film_limits, &limits)
                    .expect("generated records must encode as Film"),
            );
        }),
        digest: &film_digest,
    });

    emit(Row {
        case: case.name,
        encoding: "telex",
        operation: "decode-syntax",
        boundary: "owned-provisional-aes-records",
        size_basis: "wire-input",
        events: case.events,
        bytes: telex.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            black_box(
                parse_telex_with_limits(black_box(&telex), &limits)
                    .expect("generated Telex must parse"),
            );
        }),
        digest: &telex_digest,
    });
    emit(Row {
        case: case.name,
        encoding: "telex",
        operation: "decode-complete",
        boundary: "owned-complete-aes-stream",
        size_basis: "wire-input",
        events: case.events,
        bytes: telex.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            let decoded = parse_telex_with_limits(black_box(&telex), &limits)
                .expect("generated Telex must parse");
            let validation = validate_telex_records_with_projection_and_limits(
                &decoded.records,
                &decoded.profile,
                decoded.projection.as_deref(),
                &[],
                &limits,
            );
            assert!(validation.valid);
            black_box((decoded, validation));
        }),
        digest: &telex_digest,
    });
    emit(Row {
        case: case.name,
        encoding: "telex",
        operation: "encode",
        boundary: "resident-records-to-validated-canonical-text",
        size_basis: "wire-output",
        events: case.events,
        bytes: telex.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            black_box(
                encode_telex_with_projection_and_limits(
                    black_box(&parsed.records),
                    parsed.profile_explicit.then_some(parsed.profile.as_str()),
                    parsed
                        .projection_explicit
                        .then_some(parsed.projection.as_deref())
                        .flatten(),
                    &limits,
                )
                .expect("generated records must encode as Telex"),
            );
        }),
        digest: &telex_digest,
    });

    emit(Row {
        case: case.name,
        encoding: "json",
        operation: "decode-syntax",
        boundary: "owned-json-value",
        size_basis: "wire-input",
        events: case.events,
        bytes: json.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            black_box(
                serde_json::from_slice::<Value>(black_box(&json))
                    .expect("generated JSON must parse"),
            );
        }),
        digest: &json_digest,
    });
    emit(Row {
        case: case.name,
        encoding: "json",
        operation: "decode-owned",
        boundary: "owned-provisional-aes-records",
        size_basis: "wire-input",
        events: case.events,
        bytes: json.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            let value = serde_json::from_slice::<Value>(black_box(&json))
                .expect("generated JSON must parse");
            black_box(json_to_records(value));
        }),
        digest: &json_digest,
    });
    emit(Row {
        case: case.name,
        encoding: "json",
        operation: "decode-complete",
        boundary: "owned-complete-aes-stream",
        size_basis: "wire-input",
        events: case.events,
        bytes: json.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            let value = serde_json::from_slice::<Value>(black_box(&json))
                .expect("generated JSON must parse");
            let records = json_to_records(value);
            let validation = validate_telex_records_with_projection_and_limits(
                &records,
                COMPLETE_PROFILE,
                None,
                &[],
                &limits,
            );
            assert!(validation.valid);
            black_box((records, validation));
        }),
        digest: &json_digest,
    });
    emit(Row {
        case: case.name,
        encoding: "json",
        operation: "encode-engine",
        boundary: "resident-json-value-to-bytes-no-aes-validation",
        size_basis: "wire-output",
        events: case.events,
        bytes: json.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            black_box(
                serde_json::to_vec(black_box(&json_value))
                    .expect("generated JSON value must encode"),
            );
        }),
        digest: &json_digest,
    });
    emit(Row {
        case: case.name,
        encoding: "json",
        operation: "encode-adapter",
        boundary: "resident-aes-records-to-json-bytes-no-aes-validation",
        size_basis: "wire-output",
        events: case.events,
        bytes: json.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            let value = records_to_json(black_box(&parsed.records));
            black_box(serde_json::to_vec(&value).expect("generated JSON value must encode"));
        }),
        digest: &json_digest,
    });
    emit(Row {
        case: case.name,
        encoding: "json",
        operation: "encode-validated-adapter",
        boundary: "resident-records-to-validated-json-bytes",
        size_basis: "wire-output",
        events: case.events,
        bytes: json.len(),
        iterations: case.iterations,
        timing: measure(case, || {
            let validation = validate_telex_records_with_projection_and_limits(
                black_box(&parsed.records),
                COMPLETE_PROFILE,
                None,
                &[],
                &limits,
            );
            assert!(validation.valid);
            let value = records_to_json(&parsed.records);
            black_box(serde_json::to_vec(&value).expect("generated JSON value must encode"));
        }),
        digest: &json_digest,
    });
}

fn workload_limits(events: usize) -> TelexLimits {
    TelexLimits {
        max_list_items: events,
        ..TelexLimits::default()
    }
}

fn measure(case: &Case, mut operation: impl FnMut()) -> Timing {
    for _ in 0..case.warmups {
        operation();
    }
    let mut samples = Vec::with_capacity(case.iterations);
    for _ in 0..case.iterations {
        let start = Instant::now();
        operation();
        samples.push(start.elapsed());
    }
    let mut ordered = samples.clone();
    ordered.sort_unstable();
    let median = ordered[ordered.len() / 2];
    let p95_index = (ordered.len() * 95).div_ceil(100).saturating_sub(1);
    Timing {
        median,
        p95: ordered[p95_index],
        samples,
    }
}

fn emit(row: Row<'_>) {
    let seconds = row.timing.median.as_secs_f64();
    let mb_per_second = row.bytes as f64 / 1_000_000.0 / seconds;
    let events_per_second = row.events as f64 / seconds;
    if raw_samples_enabled() {
        println!(
            "# samples case={};encoding={};operation={};unit=ms;values={}",
            row.case,
            row.encoding,
            row.operation,
            format_samples(&row.timing.samples),
        );
    }
    println!(
        "{},{},{},{},{},{},{},{},{:.3},{:.3},{:.2},{:.0},{}",
        row.case,
        row.encoding,
        row.operation,
        row.boundary,
        row.size_basis,
        row.events,
        row.bytes,
        row.iterations,
        milliseconds(row.timing.median),
        milliseconds(row.timing.p95),
        mb_per_second,
        events_per_second,
        row.digest,
    );
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn raw_samples_enabled() -> bool {
    env::var("AES_BENCHMARK_RAW_SAMPLES").as_deref() == Ok("1")
}

fn format_samples(samples: &[Duration]) -> String {
    samples
        .iter()
        .map(|sample| format!("{:.3}", milliseconds(*sample)))
        .collect::<Vec<_>>()
        .join("|")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn records_to_json(records: &[TelexRecord]) -> Value {
    Value::Array(records.iter().map(record_to_json).collect())
}

fn record_to_json(record: &TelexRecord) -> Value {
    let mut object = Map::new();
    for (name, value) in record.fields() {
        object.insert(name.clone(), Value::String(value.clone()));
    }
    if let Some(datatype) = record.datatype() {
        object.insert(
            "datatype".to_owned(),
            Value::String(datatype.datatype.clone()),
        );
        object.insert(
            "generics".to_owned(),
            Value::Array(datatype.generics.iter().map(generic_to_json).collect()),
        );
        object.insert(
            "clarifiers".to_owned(),
            Value::Array(datatype.clarifiers.iter().map(clarifier_to_json).collect()),
        );
    }
    Value::Object(object)
}

fn descriptor_to_json(descriptor: &DatatypeDescriptor) -> Value {
    let mut object = Map::new();
    object.insert(
        "datatype".to_owned(),
        Value::String(descriptor.datatype.clone()),
    );
    object.insert(
        "generics".to_owned(),
        Value::Array(descriptor.generics.iter().map(generic_to_json).collect()),
    );
    object.insert(
        "clarifiers".to_owned(),
        Value::Array(
            descriptor
                .clarifiers
                .iter()
                .map(clarifier_to_json)
                .collect(),
        ),
    );
    Value::Object(object)
}

fn generic_to_json(generic: &GenericArgument) -> Value {
    match generic {
        GenericArgument::Datatype(descriptor) => descriptor_to_json(descriptor),
        GenericArgument::NumberLiteral(value) => tagged_value("NumberLiteral", value),
    }
}

fn clarifier_to_json(clarifier: &DatatypeClarifier) -> Value {
    let kind = match clarifier.kind {
        ClarifierKind::StringLiteral => "StringLiteral",
        ClarifierKind::NumberLiteral => "NumberLiteral",
    };
    tagged_value(kind, &clarifier.value)
}

fn tagged_value(kind: &str, value: &str) -> Value {
    let mut object = Map::new();
    object.insert("kind".to_owned(), Value::String(kind.to_owned()));
    object.insert("value".to_owned(), Value::String(value.to_owned()));
    Value::Object(object)
}

fn json_to_records(value: Value) -> Vec<TelexRecord> {
    value
        .as_array()
        .expect("portable JSON root must be an array")
        .iter()
        .map(json_to_record)
        .collect()
}

fn json_to_record(value: &Value) -> TelexRecord {
    let object = value
        .as_object()
        .expect("portable JSON record must be an object");
    let mut fields = Vec::new();
    for name in [
        "header", "path", "kind", "datatype", "identity", "value", "origin", "span",
    ] {
        if let Some(value) = object.get(name) {
            fields.push((name.to_owned(), json_string(value, name).to_owned()));
        }
    }
    for (name, value) in object {
        if !matches!(
            name.as_str(),
            "header"
                | "path"
                | "kind"
                | "datatype"
                | "generics"
                | "clarifiers"
                | "identity"
                | "value"
                | "origin"
                | "span"
        ) {
            fields.push((name.clone(), json_string(value, name).to_owned()));
        }
    }

    match object.get("datatype") {
        Some(_) => TelexRecord::with_datatype(fields, json_to_descriptor(object)),
        None => TelexRecord::new(fields),
    }
}

fn json_to_descriptor(object: &Map<String, Value>) -> DatatypeDescriptor {
    let datatype = json_string(
        object
            .get("datatype")
            .expect("datatype descriptor must have datatype"),
        "datatype",
    )
    .to_owned();
    let generics = object
        .get("generics")
        .expect("datatype descriptor must have generics")
        .as_array()
        .expect("datatype generics must be an array")
        .iter()
        .map(|generic| {
            let generic = generic
                .as_object()
                .expect("datatype generic must be an object");
            if generic.contains_key("datatype") {
                GenericArgument::Datatype(json_to_descriptor(generic))
            } else {
                assert_eq!(
                    json_string(
                        generic.get("kind").expect("numeric generic must have kind"),
                        "kind",
                    ),
                    "NumberLiteral"
                );
                GenericArgument::NumberLiteral(
                    json_string(
                        generic
                            .get("value")
                            .expect("numeric generic must have value"),
                        "value",
                    )
                    .to_owned(),
                )
            }
        })
        .collect();
    let clarifiers = object
        .get("clarifiers")
        .expect("datatype descriptor must have clarifiers")
        .as_array()
        .expect("datatype clarifiers must be an array")
        .iter()
        .map(|clarifier| {
            let clarifier = clarifier
                .as_object()
                .expect("datatype clarifier must be an object");
            let kind = match json_string(
                clarifier
                    .get("kind")
                    .expect("datatype clarifier must have kind"),
                "kind",
            ) {
                "StringLiteral" => ClarifierKind::StringLiteral,
                "NumberLiteral" => ClarifierKind::NumberLiteral,
                other => panic!("unsupported clarifier kind {other}"),
            };
            DatatypeClarifier {
                kind,
                value: json_string(
                    clarifier
                        .get("value")
                        .expect("datatype clarifier must have value"),
                    "value",
                )
                .to_owned(),
            }
        })
        .collect();
    DatatypeDescriptor {
        datatype,
        generics,
        clarifiers,
    }
}

fn json_string<'a>(value: &'a Value, field: &str) -> &'a str {
    value
        .as_str()
        .unwrap_or_else(|| panic!("portable JSON field {field} must be a string"))
}

fn build_telex(event_count: usize, payload: Payload) -> String {
    assert!((1..=100_000).contains(&event_count));
    let mut output = String::with_capacity(event_count * 60);
    output.push_str("telex.aes=1\n\npath=$.items\nkind=ListNode\ndatatype=list<string>\n");
    let repeated_payload = match payload {
        Payload::MixedShort => None,
        Payload::LargeAscii => Some("abcdefghij".repeat(64)),
        // Each ten-byte canonical Telex group decodes to five bytes. Doubling
        // the repetition count keeps decoded payload ownership equal to the
        // 640-byte large-ASCII control while exposing dense escape handling.
        Payload::EscapeHeavy => Some(r"\\\t\n\r\0".repeat(128)),
        Payload::LargeUtf8 => Some("café-日本-🙂-".repeat(32)),
    };
    for index in 0..event_count - 1 {
        output.push_str("\npath=$.items[");
        output.push_str(&index.to_string());
        output.push_str("]\n");
        if let Some(value) = &repeated_payload {
            output.push_str("kind=StringLiteral\nvalue=");
            output.push_str(value);
            output.push('\n');
            continue;
        }
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
