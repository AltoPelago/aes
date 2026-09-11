use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use aes_telex::film::{
    FilmError, FilmLimits, FilmStream, decode_film_with_limits, encode_film_with_limits,
    film_to_telex, telex_to_film,
};
use aes_telex::{
    ClarifierKind, DatatypeClarifier, DatatypeDescriptor, GenericArgument, TelexLimits, TelexRecord,
};
use serde_json::{Map, Value, json};

#[test]
fn passes_mutable_film_v1_vectors() {
    let manifest_path = selected_manifest(
        "FILM_CTS_MANIFEST",
        "../../conformance/film/v1/film-cts.v1.json",
    );
    let manifest = read_json(&manifest_path);
    assert_eq!(manifest["meta"]["version"], "0.1.0-dev");
    assert_eq!(manifest["meta"]["status"], "draft");
    assert_eq!(manifest["meta"]["lane"], "aes-film");
    assert_eq!(manifest["meta"]["format"], "film.aes");
    assert_eq!(manifest["meta"]["format_version"], "1");
    assert_eq!(manifest["meta"]["event_contract"], "aes.events.v1");
    assert_eq!(manifest["meta"]["byte_encoding"], "lowercase-hex");
    assert_eq!(manifest["meta"]["snapshot_id"], Value::Null);
    assert_eq!(manifest["meta"]["spec_snapshot_id"], Value::Null);

    let mut seen = HashSet::new();
    let mut count = 0_usize;
    for suite_ref in manifest["suites"]
        .as_array()
        .expect("Film manifest suites must be an array")
    {
        let suite_path = manifest_path
            .parent()
            .expect("Film manifest must have a parent")
            .join(
                suite_ref["file"]
                    .as_str()
                    .expect("Film suite file must be a string"),
            );
        let suite = read_json(&suite_path);
        assert_eq!(suite["id"], suite_ref["id"], "Film suite id mismatch");
        assert_eq!(suite["meta"]["lane"], manifest["meta"]["lane"]);
        assert_eq!(
            suite["meta"]["event_contract"],
            manifest["meta"]["event_contract"]
        );
        for vector in suite["tests"]
            .as_array()
            .expect("Film suite tests must be an array")
        {
            let id = vector["id"]
                .as_str()
                .expect("Film vector id must be a string");
            assert!(seen.insert(id.to_owned()), "duplicate Film vector id: {id}");
            run_vector(id, vector);
            count += 1;
        }
    }
    assert_eq!(count, 68, "unexpected mutable Film vector count");
}

fn run_vector(id: &str, vector: &Value) {
    match vector["operation"].as_str() {
        Some("decode") => run_decode_vector(id, vector),
        Some("encode") => run_encode_vector(id, vector),
        Some("transcode") => run_transcode_vector(id, vector),
        operation => panic!("{id}: unsupported Film operation {operation:?}"),
    }
}

fn run_decode_vector(id: &str, vector: &Value) {
    let bytes = decode_hex(required_string(&vector["input"], "film_hex"))
        .unwrap_or_else(|error| panic!("{id}: invalid CTS hex: {error}"));
    let registered = registered_fields(&vector["input"]);
    let (film_limits, aes_limits) = vector_limits(&vector["input"]);
    let expected = &vector["expected"];
    match decode_film_with_limits(&bytes, &registered, &film_limits, &aes_limits) {
        Ok(stream) => {
            assert_eq!(expected["ok"], true, "{id}: expected decode failure");
            if !expected["stream"].is_null() {
                assert_eq!(stream_json(&stream), expected["stream"], "{id}");
            }
            if let Some(canonical) = expected["canonical_hex"].as_str() {
                let encoded =
                    encode_film_with_limits(&stream, &registered, &film_limits, &aes_limits)
                        .unwrap_or_else(|error| {
                            panic!("{id}: canonical re-encode failed: {error}")
                        });
                assert_eq!(lower_hex(&encoded), canonical, "{id}");
            }
        }
        Err(error) => assert_expected_error(id, expected, &error),
    }
}

fn run_encode_vector(id: &str, vector: &Value) {
    let stream = stream_from_json(&vector["input"]["stream"]);
    let registered = registered_fields(&vector["input"]);
    let (film_limits, aes_limits) = vector_limits(&vector["input"]);
    let expected = &vector["expected"];
    match encode_film_with_limits(&stream, &registered, &film_limits, &aes_limits) {
        Ok(bytes) => {
            assert_eq!(expected["ok"], true, "{id}: expected encode failure");
            assert_eq!(lower_hex(&bytes), expected["film_hex"], "{id}");
        }
        Err(error) => assert_expected_error(id, expected, &error),
    }
}

fn run_transcode_vector(id: &str, vector: &Value) {
    let telex = required_string(&vector["input"], "telex");
    let registered = registered_fields(&vector["input"]);
    let film = telex_to_film(telex, &registered)
        .unwrap_or_else(|error| panic!("{id}: Telex-to-Film failed: {error}"));
    assert_eq!(lower_hex(&film), vector["expected"]["film_hex"], "{id}");
    let round_trip = film_to_telex(&film, &registered)
        .unwrap_or_else(|error| panic!("{id}: Film-to-Telex failed: {error}"));
    assert_eq!(round_trip, vector["expected"]["telex"], "{id}");
}

fn assert_expected_error(id: &str, expected: &Value, error: &FilmError) {
    assert_eq!(
        expected["ok"], false,
        "{id}: unexpected Film error: {error}"
    );
    if let Some(code) = expected["error"]["code"].as_str() {
        assert_eq!(error.code, code, "{id}: {error}");
    }
    if expected["error"]["stage"] == "aes" {
        assert_eq!(error.code, "FILM_AES_INVALID", "{id}: {error}");
    }
    if let Some(component) = expected["error"]["component"].as_str() {
        assert_eq!(error.component, component, "{id}");
    }
    if !expected["error"]["record"].is_null() {
        assert_eq!(json!(error.record), expected["error"]["record"], "{id}");
    }
    if let Some(expected_codes) = expected["error"]["diagnostic_codes"].as_array() {
        let mut actual = error
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>();
        actual.sort_unstable();
        let mut expected = expected_codes
            .iter()
            .map(|code| code.as_str().expect("diagnostic code must be a string"))
            .collect::<Vec<_>>();
        expected.sort_unstable();
        assert_eq!(actual, expected, "{id}");
    }
}

fn vector_limits(input: &Value) -> (FilmLimits, TelexLimits) {
    let mut film = FilmLimits::default();
    let mut aes = TelexLimits::default();
    let Some(values) = input["limits"].as_object() else {
        return (film, aes);
    };
    for (name, value) in values {
        let value = value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .expect("Film CTS limit must be a non-negative platform-sized integer");
        match name.as_str() {
            "max_input_bytes" => film.max_input_bytes = value,
            "max_record_bytes" => film.max_record_bytes = value,
            "max_field_bytes" => film.max_field_bytes = value,
            "max_buffered_bytes" => film.max_buffered_bytes = value,
            "max_events" => aes.max_events = value,
            "max_path_depth" => aes.max_path_depth = value,
            "max_path_characters" => aes.max_path_characters = value,
            "max_attribute_depth" => aes.max_attribute_depth = value,
            "max_value_nesting_depth" => aes.max_value_nesting_depth = value,
            "max_string_codepoints" => aes.max_string_codepoints = value,
            "max_key_segment_codepoints" => aes.max_key_segment_codepoints = value,
            "max_list_items" => aes.max_list_items = value,
            "max_tuple_items" => aes.max_tuple_items = value,
            "max_generic_depth" => aes.max_generic_depth = value,
            "max_generic_arguments" => aes.max_generic_arguments = value,
            "max_clarifier_values" => aes.max_clarifier_values = value,
            "max_datatype_components" => aes.max_datatype_components = value,
            _ => panic!("unknown Film CTS limit: {name}"),
        }
    }
    (film, aes)
}

fn registered_fields(input: &Value) -> Vec<&str> {
    input["registered_fields"]
        .as_array()
        .map(|fields| {
            fields
                .iter()
                .map(|field| field.as_str().expect("registered field must be a string"))
                .collect()
        })
        .unwrap_or_default()
}

fn stream_from_json(value: &Value) -> FilmStream {
    FilmStream {
        profile: required_string(value, "profile").to_owned(),
        profile_explicit: value["profile_explicit"]
            .as_bool()
            .expect("profile_explicit must be a boolean"),
        projection: value["projection"].as_str().map(str::to_owned),
        projection_explicit: value["projection_explicit"]
            .as_bool()
            .expect("projection_explicit must be a boolean"),
        records: value["records"]
            .as_array()
            .expect("records must be an array")
            .iter()
            .map(record_from_json)
            .collect(),
    }
}

fn stream_json(stream: &FilmStream) -> Value {
    json!({
        "profile": stream.profile,
        "profile_explicit": stream.profile_explicit,
        "projection": stream.projection,
        "projection_explicit": stream.projection_explicit,
        "records": stream.records.iter().map(record_json).collect::<Vec<_>>(),
    })
}

fn record_from_json(value: &Value) -> TelexRecord {
    let object = value
        .as_object()
        .expect("Film CTS record must be an object");
    let fields = object
        .iter()
        .filter(|(name, _)| name.as_str() != "generics" && name.as_str() != "clarifiers")
        .map(|(name, value)| {
            (
                name.clone(),
                value
                    .as_str()
                    .unwrap_or_else(|| panic!("Film CTS record field {name} must be a string"))
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
                    .expect("datatype records require generics")
                    .iter()
                    .map(generic_from_json)
                    .collect(),
                clarifiers: object
                    .get("clarifiers")
                    .and_then(Value::as_array)
                    .expect("datatype records require clarifiers")
                    .iter()
                    .map(clarifier_from_json)
                    .collect(),
            },
        ),
        None => TelexRecord::new(fields),
    }
}

fn generic_from_json(value: &Value) -> GenericArgument {
    if value.get("datatype").is_some() {
        return GenericArgument::Datatype(datatype_from_json(value));
    }
    assert_eq!(value["kind"], "NumberLiteral");
    GenericArgument::NumberLiteral(required_string(value, "value").to_owned())
}

fn datatype_from_json(value: &Value) -> DatatypeDescriptor {
    DatatypeDescriptor {
        datatype: required_string(value, "datatype").to_owned(),
        generics: value["generics"]
            .as_array()
            .expect("nested datatype generics must be an array")
            .iter()
            .map(generic_from_json)
            .collect(),
        clarifiers: value["clarifiers"]
            .as_array()
            .expect("nested datatype clarifiers must be an array")
            .iter()
            .map(clarifier_from_json)
            .collect(),
    }
}

fn clarifier_from_json(value: &Value) -> DatatypeClarifier {
    DatatypeClarifier {
        kind: match required_string(value, "kind") {
            "StringLiteral" => ClarifierKind::StringLiteral,
            "NumberLiteral" => ClarifierKind::NumberLiteral,
            kind => panic!("unsupported Film CTS clarifier kind: {kind}"),
        },
        value: required_string(value, "value").to_owned(),
    }
}

fn record_json(record: &TelexRecord) -> Value {
    let mut fields = record
        .fields()
        .iter()
        .map(|(field, value)| (field.clone(), Value::String(value.clone())))
        .collect::<Map<_, _>>();
    if let Some(datatype) = record.datatype() {
        fields.insert(
            "generics".to_owned(),
            Value::Array(datatype.generics.iter().map(generic_json).collect()),
        );
        fields.insert(
            "clarifiers".to_owned(),
            Value::Array(datatype.clarifiers.iter().map(clarifier_json).collect()),
        );
    }
    Value::Object(fields)
}

fn generic_json(value: &GenericArgument) -> Value {
    match value {
        GenericArgument::Datatype(datatype) => datatype_json(datatype),
        GenericArgument::NumberLiteral(value) => {
            json!({ "kind": "NumberLiteral", "value": value })
        }
    }
}

fn datatype_json(value: &DatatypeDescriptor) -> Value {
    json!({
        "datatype": value.datatype,
        "generics": value.generics.iter().map(generic_json).collect::<Vec<_>>(),
        "clarifiers": value.clarifiers.iter().map(clarifier_json).collect::<Vec<_>>(),
    })
}

fn clarifier_json(value: &DatatypeClarifier) -> Value {
    json!({
        "kind": match value.kind {
            ClarifierKind::StringLiteral => "StringLiteral",
            ClarifierKind::NumberLiteral => "NumberLiteral",
        },
        "value": value.value,
    })
}

fn required_string<'a>(value: &'a Value, name: &str) -> &'a str {
    value[name]
        .as_str()
        .unwrap_or_else(|| panic!("{name} must be a string"))
}

fn decode_hex(input: &str) -> Result<Vec<u8>, String> {
    if !input.len().is_multiple_of(2) {
        return Err("hex byte strings require an even number of digits".to_owned());
    }
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).map_err(|error| error.to_string())?;
            u8::from_str_radix(pair, 16).map_err(|error| error.to_string())
        })
        .collect()
}

fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn selected_manifest(variable: &str, fallback: &str) -> PathBuf {
    std::env::var_os(variable).map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join(fallback),
        PathBuf::from,
    )
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(
        &fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display())),
    )
    .unwrap_or_else(|error| panic!("{} must contain JSON: {error}", path.display()))
}
