use std::fs;
use std::path::{Path, PathBuf};

use aes_telex::{
    ClarifierKind, DatatypeClarifier, DatatypeDescriptor, GenericArgument, TelexLimits,
    TelexRecord, canonicalize_telex_with_limits, parse_telex_with_limits,
    validate_telex_records_with_projection_and_limits, validate_telex_with_limits,
};
use serde_json::{Map, Value, json};

#[test]
fn passes_selected_v0_telex_vectors() {
    let manifest_path = selected_manifest(
        "TELEX_CTS_MANIFEST",
        "../../conformance/telex/v0/telex-cts.v0.json",
    );
    let manifest = read_json(&manifest_path);
    let released = manifest["meta"]["status"] == "released";
    if released {
        assert_eq!(manifest["meta"]["snapshot_id"], "telex-cts-v0-snapshot-0.1");
        assert_eq!(
            manifest["meta"]["spec_snapshot_id"],
            "telex-specs-v0-snapshot-0.1"
        );
    } else {
        assert_eq!(manifest["meta"]["status"], "draft");
        assert_eq!(manifest["meta"]["snapshot_id"], Value::Null);
        assert_eq!(manifest["meta"]["spec_snapshot_id"], Value::Null);
    }
    assert_eq!(manifest["meta"]["event_contract"], "aes.events.v0");
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
        assert_eq!(
            suite["meta"]["event_contract"], manifest["meta"]["event_contract"],
            "suite event contract mismatch"
        );
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

    assert_eq!(
        count,
        if released { 50 } else { 88 },
        "unexpected v0 vector count"
    );
}

#[test]
fn passes_published_portable_aes_event_vectors() {
    let manifest_path = selected_manifest(
        "AES_EVENTS_CTS_MANIFEST",
        "../../../../aeonite-org/aeonite-cts/cts/aes/v0/aes-events-cts.v0.snapshot-0.1.json",
    );
    let manifest = read_json(&manifest_path);
    assert_eq!(manifest["meta"]["status"], "released");
    assert_eq!(manifest["meta"]["lane"], "aes-events");
    assert_eq!(manifest["meta"]["event_contract"], "aes.events.v0");
    assert_eq!(
        manifest["meta"]["snapshot_id"],
        "aes-events-cts-v0-snapshot-0.1"
    );

    let mut seen = std::collections::HashSet::new();
    let mut count = 0_usize;
    for suite_ref in manifest["suites"]
        .as_array()
        .expect("manifest suites must be an array")
    {
        let suite_path = manifest_path
            .parent()
            .expect("manifest must have a parent")
            .join(
                suite_ref["file"]
                    .as_str()
                    .expect("suite file must be a string"),
            );
        let suite = read_json(&suite_path);
        assert_eq!(suite["id"], suite_ref["id"], "suite id mismatch");
        for vector in suite["tests"]
            .as_array()
            .expect("suite tests must be an array")
        {
            let id = vector["id"].as_str().expect("vector id must be a string");
            assert!(seen.insert(id.to_owned()), "duplicate vector id: {id}");
            run_aes_event_vector(id, vector);
            count += 1;
        }
    }
    assert_eq!(count, 38, "unexpected portable AES event vector count");
}

fn run_aes_event_vector(id: &str, vector: &Value) {
    assert_eq!(vector["operation"], "validate", "{id}");
    let records = vector["input"]["records"]
        .as_array()
        .expect("records must be an array")
        .iter()
        .map(record_from_json)
        .collect::<Vec<_>>();
    let profile = vector["input"]["profile"]
        .as_str()
        .unwrap_or("aes.complete.v0");
    let projection = vector["input"]["projection"].as_str();
    let registered = vector["input"]["registered_fields"]
        .as_array()
        .map(|fields| {
            fields
                .iter()
                .map(|field| field.as_str().expect("registered field must be a string"))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let result = validate_telex_records_with_projection_and_limits(
        &records,
        profile,
        projection,
        &registered,
        &TelexLimits::default(),
    );
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
    let limits = vector_limits(vector);
    match parse_telex_with_limits(input, &limits) {
        Ok(parsed) => {
            assert_ne!(expected["ok"], false, "{id}: expected syntax failure");
            let records = parsed.records.iter().map(record_json).collect::<Vec<_>>();
            let actual = json!({
                "ok": true,
                "version": parsed.version,
                "profile": parsed.profile,
                "profile_explicit": parsed.profile_explicit,
                "projection": parsed.projection,
                "projection_explicit": parsed.projection_explicit,
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
    let limits = vector_limits(vector);
    match canonicalize_telex_with_limits(input, &limits) {
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
    let limits = vector_limits(vector);
    let result = validate_telex_with_limits(input, &registered, &limits)
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

fn vector_limits(vector: &Value) -> TelexLimits {
    let mut limits = TelexLimits::default();
    let Some(values) = vector["input"]["limits"].as_object() else {
        return limits;
    };
    for (name, value) in values {
        let value = value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .expect("limit must be a non-negative platform-sized integer");
        match name.as_str() {
            "max_input_bytes" => limits.max_input_bytes = value,
            "max_line_bytes" => limits.max_line_bytes = value,
            "max_fields_per_event" => limits.max_fields_per_event = value,
            "max_events" => limits.max_events = value,
            "max_decoded_payload_bytes" => limits.max_decoded_payload_bytes = value,
            "max_path_depth" => limits.max_path_depth = value,
            "max_path_characters" => limits.max_path_characters = value,
            "max_generic_depth" => limits.max_generic_depth = value,
            "max_generic_arguments" => limits.max_generic_arguments = value,
            "max_clarifier_values" => limits.max_clarifier_values = value,
            "max_datatype_components" => limits.max_datatype_components = value,
            _ => panic!("unknown vector limit: {name}"),
        }
    }
    limits
}

fn record_json(record: &TelexRecord) -> Value {
    let mut fields = record
        .fields()
        .iter()
        .map(|(field, value)| (field.clone(), Value::String(value.clone())))
        .collect::<Map<_, _>>();
    if let Some(datatype) = record.datatype() {
        fields.insert("generics".to_owned(), generics_json(datatype));
        fields.insert("clarifiers".to_owned(), clarifiers_json(datatype));
    }
    Value::Object(fields)
}

fn datatype_json(datatype: &DatatypeDescriptor) -> Value {
    json!({
        "datatype": datatype.datatype,
        "generics": datatype.generics.iter().map(|argument| match argument {
            GenericArgument::Datatype(nested) => datatype_json(nested),
            GenericArgument::NumberLiteral(value) => {
                json!({ "kind": "NumberLiteral", "value": value })
            }
        }).collect::<Vec<_>>(),
        "clarifiers": datatype.clarifiers.iter().map(|clarifier| json!({
            "kind": match clarifier.kind {
                ClarifierKind::StringLiteral => "StringLiteral",
                ClarifierKind::NumberLiteral => "NumberLiteral",
            },
            "value": clarifier.value,
        })).collect::<Vec<_>>(),
    })
}

fn generics_json(datatype: &DatatypeDescriptor) -> Value {
    Value::Array(
        datatype
            .generics
            .iter()
            .map(|argument| match argument {
                GenericArgument::Datatype(nested) => datatype_json(nested),
                GenericArgument::NumberLiteral(value) => {
                    json!({ "kind": "NumberLiteral", "value": value })
                }
            })
            .collect(),
    )
}

fn clarifiers_json(datatype: &DatatypeDescriptor) -> Value {
    Value::Array(
        datatype
            .clarifiers
            .iter()
            .map(|clarifier| {
                json!({
                    "kind": match clarifier.kind {
                        ClarifierKind::StringLiteral => "StringLiteral",
                        ClarifierKind::NumberLiteral => "NumberLiteral",
                    },
                    "value": clarifier.value,
                })
            })
            .collect(),
    )
}

fn record_from_json(value: &Value) -> TelexRecord {
    let object = value.as_object().expect("AES record must be an object");
    let fields = object
        .iter()
        .filter(|(name, _)| name.as_str() != "generics" && name.as_str() != "clarifiers")
        .map(|(name, value)| {
            (
                name.clone(),
                value
                    .as_str()
                    .unwrap_or_else(|| panic!("AES record field {name} must be a string"))
                    .to_owned(),
            )
        })
        .collect::<Vec<_>>();
    match object.get("datatype").and_then(Value::as_str) {
        Some(datatype) => TelexRecord::with_datatype(
            fields,
            DatatypeDescriptor {
                datatype: datatype.to_owned(),
                generics: object
                    .get("generics")
                    .and_then(Value::as_array)
                    .map(|values| values.iter().map(generic_from_json).collect())
                    .unwrap_or_default(),
                clarifiers: object
                    .get("clarifiers")
                    .and_then(Value::as_array)
                    .map(|values| values.iter().map(clarifier_from_json).collect())
                    .unwrap_or_default(),
            },
        ),
        None => TelexRecord::new(fields),
    }
}

fn generic_from_json(value: &Value) -> GenericArgument {
    if value["kind"] == "NumberLiteral" {
        return GenericArgument::NumberLiteral(
            value["value"]
                .as_str()
                .expect("number generic value must be a string")
                .to_owned(),
        );
    }
    GenericArgument::Datatype(datatype_from_json(value))
}

fn datatype_from_json(value: &Value) -> DatatypeDescriptor {
    DatatypeDescriptor {
        datatype: value["datatype"]
            .as_str()
            .expect("nested datatype name must be a string")
            .to_owned(),
        generics: value["generics"]
            .as_array()
            .map(|values| values.iter().map(generic_from_json).collect())
            .unwrap_or_default(),
        clarifiers: value["clarifiers"]
            .as_array()
            .map(|values| values.iter().map(clarifier_from_json).collect())
            .unwrap_or_default(),
    }
}

fn clarifier_from_json(value: &Value) -> DatatypeClarifier {
    DatatypeClarifier {
        kind: match value["kind"].as_str() {
            Some("StringLiteral") => ClarifierKind::StringLiteral,
            Some("NumberLiteral") => ClarifierKind::NumberLiteral,
            kind => panic!("unsupported clarifier kind: {kind:?}"),
        },
        value: value["value"]
            .as_str()
            .expect("clarifier value must be a string")
            .to_owned(),
    }
}

fn selected_manifest(environment_name: &str, default_relative: &str) -> PathBuf {
    match std::env::var_os(environment_name) {
        Some(path) => {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                path
            } else {
                std::env::current_dir()
                    .expect("current directory must be available")
                    .join(path)
            }
        }
        None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(default_relative),
    }
}

fn read_json(path: &Path) -> Value {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}
