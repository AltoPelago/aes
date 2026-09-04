use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use aes_telex::{TelexRecord, canonicalize_telex, parse_telex, validate_telex};
use serde_json::{Value, json};

#[test]
fn passes_draft_0_telex_vectors() {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/telex/v0/telex-cts.v0.json");
    let manifest = read_json(&manifest_path);
    let suites = manifest["suites"]
        .as_array()
        .expect("manifest suites must be an array");
    let mut seen = std::collections::HashSet::new();
    let mut count = 0_usize;

    for suite_ref in suites {
        let relative = suite_ref["file"]
            .as_str()
            .expect("suite file must be a string");
        let suite_path = manifest_path
            .parent()
            .expect("manifest must have a parent")
            .join(relative);
        let suite = read_json(&suite_path);
        assert_eq!(suite["id"], suite_ref["id"], "suite id mismatch");
        for vector in suite["tests"]
            .as_array()
            .expect("suite tests must be an array")
        {
            let id = vector["id"].as_str().expect("vector id must be a string");
            assert!(seen.insert(id.to_owned()), "duplicate vector id: {id}");
            run_vector(id, vector);
            count += 1;
        }
    }

    assert_eq!(count, 33, "unexpected Draft 0 vector count");
}

fn run_vector(id: &str, vector: &Value) {
    match vector["operation"].as_str() {
        Some("parse") => run_parse_vector(id, vector),
        Some("canonicalize") => run_canonicalize_vector(id, vector),
        Some("validate") => run_validate_vector(id, vector),
        operation => panic!("{id}: unsupported operation {operation:?}"),
    }
}

fn run_parse_vector(id: &str, vector: &Value) {
    let input = vector["input"]["telex"]
        .as_str()
        .expect("parse input must be a string");
    let expected = &vector["expected"];
    match parse_telex(input) {
        Ok(parsed) => {
            assert_ne!(expected["ok"], false, "{id}: expected syntax failure");
            let records = parsed.records.iter().map(record_json).collect::<Vec<_>>();
            let actual = json!({
                "ok": true,
                "version": parsed.version,
                "profile": parsed.profile,
                "profile_explicit": parsed.profile_explicit,
                "canonical": parsed.canonical,
                "records": records,
            });
            assert_eq!(&actual, expected, "{id}");
        }
        Err(error) => {
            assert_eq!(expected["ok"], false, "{id}: unexpected error: {error}");
            assert_eq!(error.code, expected["error"]["code"], "{id}");
            assert_eq!(json!(error.line), expected["error"]["line"], "{id}");
        }
    }
}

fn run_canonicalize_vector(id: &str, vector: &Value) {
    let input = vector["input"]["telex"]
        .as_str()
        .expect("canonicalize input must be a string");
    let expected = &vector["expected"];
    match canonicalize_telex(input) {
        Ok(telex) => {
            let actual = json!({ "ok": true, "telex": telex });
            assert_eq!(&actual, expected, "{id}");
        }
        Err(error) => {
            assert_eq!(expected["ok"], false, "{id}: unexpected error: {error}");
            assert_eq!(error.code, expected["error"]["code"], "{id}");
            assert_eq!(json!(error.line), expected["error"]["line"], "{id}");
        }
    }
}

fn run_validate_vector(id: &str, vector: &Value) {
    let input = vector["input"]["telex"]
        .as_str()
        .expect("validate input must be a string");
    let registered = vector["input"]["registered_fields"]
        .as_array()
        .map(|fields| {
            fields
                .iter()
                .map(|field| field.as_str().expect("registered field must be a string"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let result = validate_telex(input, &registered)
        .unwrap_or_else(|error| panic!("{id}: unexpected syntax error: {error}"));
    let mut actual_codes = result
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();
    actual_codes.sort_unstable();
    let mut expected_codes = vector["expected"]["diagnostic_codes"]
        .as_array()
        .expect("diagnostic_codes must be an array")
        .iter()
        .map(|code| code.as_str().expect("diagnostic code must be a string"))
        .collect::<Vec<_>>();
    expected_codes.sort_unstable();

    assert_eq!(result.valid, vector["expected"]["valid"], "{id}");
    assert_eq!(result.profile, vector["expected"]["profile"], "{id}");
    assert_eq!(actual_codes, expected_codes, "{id}");
}

fn record_json(record: &TelexRecord) -> Value {
    let fields = record.fields().iter().cloned().collect::<BTreeMap<_, _>>();
    json!(fields)
}

fn read_json(path: &Path) -> Value {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}
