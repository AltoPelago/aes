use std::error::Error;
use std::fs;
use std::path::PathBuf;

use aes_telex::{AesSourceArtifact, TelexRecord, audit_aes_source_provenance};
use serde_json::Value;

#[test]
fn passes_aes_provenance_candidate_vectors() -> Result<(), Box<dyn Error>> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/provenance/v0/aes-provenance-cts.v0.json");
    let manifest: Value = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    assert_eq!(manifest["meta"]["lane"], "aes-provenance");
    let mut count = 0_usize;
    for suite_ref in manifest["suites"].as_array().ok_or("missing suites")? {
        let suite_path = manifest_path
            .parent()
            .ok_or("manifest has no parent")?
            .join(suite_ref["file"].as_str().ok_or("missing suite file")?);
        let suite: Value = serde_json::from_str(&fs::read_to_string(suite_path)?)?;
        for vector in suite["tests"].as_array().ok_or("missing tests")? {
            count += 1;
            run_vector(vector)?;
        }
    }
    assert_eq!(count, 11);
    Ok(())
}

fn run_vector(vector: &Value) -> Result<(), Box<dyn Error>> {
    let input = vector["input"].as_object().ok_or("missing input")?;
    let expected = vector["expected"].as_object().ok_or("missing expected")?;
    let records = input["records"]
        .as_array()
        .ok_or("missing records")?
        .iter()
        .map(record)
        .collect::<Result<Vec<_>, _>>()?;
    let artifacts = input["artifacts"]
        .as_object()
        .ok_or("missing artifacts")?
        .iter()
        .map(|(origin, encoded)| {
            Ok(AesSourceArtifact {
                origin: origin.clone(),
                bytes: decode_base64(encoded.as_str().ok_or("artifact must be base64")?)?,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let result = audit_aes_source_provenance(
        &records,
        &artifacts,
        input
            .get("require_all_records")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        input
            .get("require_available")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    );
    assert_eq!(
        result.valid,
        expected["valid"].as_bool().ok_or("missing valid")?
    );
    assert_eq!(
        result.complete,
        expected["complete"].as_bool().ok_or("missing complete")?
    );
    assert_eq!(
        result.verified_origins,
        expected["verified_origins"]
            .as_array()
            .ok_or("missing verified_origins")?
            .iter()
            .map(|value| value.as_str().unwrap_or_default().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        result
            .diagnostics
            .iter()
            .map(|item| item.code)
            .collect::<Vec<_>>(),
        expected["diagnostic_codes"]
            .as_array()
            .ok_or("missing diagnostic_codes")?
            .iter()
            .map(|value| value.as_str().unwrap_or_default())
            .collect::<Vec<_>>()
    );
    Ok(())
}

fn record(value: &Value) -> Result<TelexRecord, Box<dyn Error>> {
    let object = value.as_object().ok_or("record must be a map")?;
    Ok(TelexRecord::new(
        object
            .iter()
            .map(|(field, value)| {
                Ok((
                    field.clone(),
                    value
                        .as_str()
                        .ok_or("record field must be a string")?
                        .to_owned(),
                ))
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?,
    ))
}

fn decode_base64(input: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if !input.len().is_multiple_of(4) {
        return Err("invalid base64 length".into());
    }
    let mut output = Vec::with_capacity(input.len() / 4 * 3);
    for chunk in input.as_bytes().chunks_exact(4) {
        let a = base64_value(chunk[0]).ok_or("invalid base64")?;
        let b = base64_value(chunk[1]).ok_or("invalid base64")?;
        let c = if chunk[2] == b'=' {
            0
        } else {
            base64_value(chunk[2]).ok_or("invalid base64")?
        };
        let d = if chunk[3] == b'=' {
            0
        } else {
            base64_value(chunk[3]).ok_or("invalid base64")?
        };
        output.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            output.push((b << 4) | (c >> 2));
        }
        if chunk[3] != b'=' {
            output.push((c << 6) | d);
        }
    }
    Ok(output)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
