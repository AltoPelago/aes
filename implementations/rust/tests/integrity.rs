use std::error::Error;
use std::fs;
use std::path::PathBuf;

use aes_telex::{
    AES_BODY_SCOPE, AES_CANONICAL_SEMANTIC_ORDER, AES_EXACT_ORDER, AES_PROVENANCE_EXCLUDED,
    AesIntegrityOptions, ClarifierKind, DatatypeClarifier, DatatypeDescriptor, GenericArgument,
    TelexLimits, TelexRecord, compute_aes_integrity_digest,
    compute_aes_integrity_digest_with_limits, encode_aes_signature_input,
};
use serde_json::{Map, Value};

#[test]
fn passes_aes_integrity_candidate_vectors() -> Result<(), Box<dyn Error>> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/integrity/v1/aes-integrity-cts.v1.json");
    let manifest: Value = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    assert_eq!(manifest["meta"]["integrity_contract"], "aes.integrity.v1");
    let suites = manifest["suites"].as_array().ok_or("missing suites")?;
    let mut count = 0_usize;
    for suite_ref in suites {
        let file = suite_ref["file"].as_str().ok_or("missing suite file")?;
        let suite_path = manifest_path
            .parent()
            .ok_or("manifest has no parent")?
            .join(file);
        let suite: Value = serde_json::from_str(&fs::read_to_string(suite_path)?)?;
        for vector in suite["tests"].as_array().ok_or("missing tests")? {
            count += 1;
            run_vector(vector)?;
        }
    }
    assert_eq!(count, 17);
    Ok(())
}

#[test]
fn canonical_and_exact_policies_remain_distinct() -> Result<(), Box<dyn Error>> {
    let records = vec![
        TelexRecord::new(vec![
            ("path".to_owned(), "$.b".to_owned()),
            ("kind".to_owned(), "NumberLiteral".to_owned()),
            ("value".to_owned(), "2".to_owned()),
        ]),
        TelexRecord::new(vec![
            ("path".to_owned(), "$.a".to_owned()),
            ("kind".to_owned(), "NumberLiteral".to_owned()),
            ("value".to_owned(), "1".to_owned()),
        ]),
    ];
    let canonical = AesIntegrityOptions::new(
        AES_CANONICAL_SEMANTIC_ORDER,
        AES_BODY_SCOPE,
        AES_PROVENANCE_EXCLUDED,
    );
    let exact = AesIntegrityOptions::new(AES_EXACT_ORDER, AES_BODY_SCOPE, AES_PROVENANCE_EXCLUDED);
    assert_ne!(
        compute_aes_integrity_digest(&records, &canonical)?.digest,
        compute_aes_integrity_digest(&records, &exact)?.digest,
    );
    Ok(())
}

#[test]
fn integrity_boundary_honors_caller_selected_telex_limits() -> Result<(), Box<dyn Error>> {
    let records = vec![TelexRecord::new(vec![
        ("path".to_owned(), "$.a".to_owned()),
        ("kind".to_owned(), "StringLiteral".to_owned()),
        ("value".to_owned(), "xx".to_owned()),
    ])];
    let options =
        AesIntegrityOptions::new(AES_EXACT_ORDER, AES_BODY_SCOPE, AES_PROVENANCE_EXCLUDED);
    assert!(compute_aes_integrity_digest(&records, &options).is_ok());

    let limits = TelexLimits {
        max_string_codepoints: 1,
        ..TelexLimits::default()
    };
    let error = compute_aes_integrity_digest_with_limits(&records, &options, &limits)
        .expect_err("selected limit should reject the integrity input");
    assert_eq!(error.code, "AES_INTEGRITY_INVALID_LOGICAL_VALUE");
    Ok(())
}

fn run_vector(vector: &Value) -> Result<(), Box<dyn Error>> {
    let id = vector["id"].as_str().ok_or("missing vector id")?;
    let operation = vector["operation"].as_str().ok_or("missing operation")?;
    let input = vector["input"].as_object().ok_or("missing input")?;
    let expected = vector["expected"].as_object().ok_or("missing expected")?;
    if operation == "signature-input" {
        let result = encode_aes_signature_input(
            required_string(input, "digest")?,
            required_string(input, "alg")?,
            required_string(input, "kid")?,
        )?;
        assert_eq!(
            lower_hex(&result.bytes),
            required_string(expected, "logical_hex")?,
            "{id}",
        );
        return Ok(());
    }
    if operation != "digest" {
        return Err(format!("unsupported operation in {id}: {operation}").into());
    }
    let records = input["records"]
        .as_array()
        .ok_or("records must be an array")?
        .iter()
        .map(record_from_json)
        .collect::<Result<Vec<_>, _>>()?;
    let mut options = AesIntegrityOptions::new(
        required_string(input, "ordering")?,
        required_string(input, "scope")?,
        required_string(input, "provenance")?,
    );
    if let Some(profile) = input.get("profile").and_then(Value::as_str) {
        options.profile = profile.to_owned();
    }
    if let Some(projection) = input.get("projection").and_then(Value::as_str) {
        options.projection = Some(projection.to_owned());
    }
    options.registered_fields = input
        .get("registered_fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .map(|field| {
                    field
                        .as_str()
                        .ok_or("registered field must be a string")
                        .map(str::to_owned)
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?
        .unwrap_or_default();

    let result = compute_aes_integrity_digest(&records, &options);
    if let Some(error_code) = expected.get("error_code").and_then(Value::as_str) {
        let error = result.expect_err("negative integrity vector unexpectedly passed");
        assert_eq!(error.code, error_code, "{id}");
        return Ok(());
    }
    let result = result?;
    assert_eq!(result.digest, required_string(expected, "digest")?, "{id}");
    if let Some(logical_hex) = expected.get("logical_hex").and_then(Value::as_str) {
        assert_eq!(lower_hex(&result.bytes), logical_hex, "{id}");
    }
    let expected_addresses = expected["record_addresses"]
        .as_array()
        .ok_or("missing record addresses")?
        .iter()
        .map(|value| value.as_str().ok_or("record address must be a string"))
        .collect::<Result<Vec<_>, _>>()?;
    let actual_addresses = result
        .records
        .iter()
        .map(|record| {
            record
                .get("header")
                .or_else(|| record.get("path"))
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    assert_eq!(actual_addresses, expected_addresses, "{id}");
    Ok(())
}

fn record_from_json(value: &Value) -> Result<TelexRecord, Box<dyn Error>> {
    let object = value.as_object().ok_or("record must be an object")?;
    let fields = object
        .iter()
        .filter(|(key, _)| key.as_str() != "generics" && key.as_str() != "clarifiers")
        .map(|(key, value)| {
            value
                .as_str()
                .map(|value| (key.clone(), value.to_owned()))
                .ok_or_else(|| format!("record field {key} must be a string"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if object.contains_key("datatype") {
        Ok(TelexRecord::with_datatype(
            fields,
            datatype_from_json(object)?,
        ))
    } else {
        Ok(TelexRecord::new(fields))
    }
}

fn datatype_from_json(object: &Map<String, Value>) -> Result<DatatypeDescriptor, Box<dyn Error>> {
    let datatype = required_string(object, "datatype")?.to_owned();
    let generics = object["generics"]
        .as_array()
        .ok_or("generics must be an array")?
        .iter()
        .map(|value| {
            let object = value.as_object().ok_or("generic must be an object")?;
            if object.contains_key("datatype") {
                Ok(GenericArgument::Datatype(datatype_from_json(object)?))
            } else if required_string(object, "kind")? == "NumberLiteral" {
                Ok(GenericArgument::NumberLiteral(
                    required_string(object, "value")?.to_owned(),
                ))
            } else {
                Err("invalid generic kind".into())
            }
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let clarifiers = object["clarifiers"]
        .as_array()
        .ok_or("clarifiers must be an array")?
        .iter()
        .map(|value| {
            let object = value.as_object().ok_or("clarifier must be an object")?;
            let kind = match required_string(object, "kind")? {
                "StringLiteral" => ClarifierKind::StringLiteral,
                "NumberLiteral" => ClarifierKind::NumberLiteral,
                _ => return Err("invalid clarifier kind".into()),
            };
            Ok(DatatypeClarifier {
                kind,
                value: required_string(object, "value")?.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    Ok(DatatypeDescriptor {
        datatype,
        generics,
        clarifiers,
    })
}

fn required_string<'a>(
    object: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, Box<dyn Error>> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string field {key}").into())
}

fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
