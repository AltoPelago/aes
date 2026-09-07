use std::error::Error;
use std::fs;
use std::path::PathBuf;

use aes_telex::{
    AES_ASP_REVISION_PRECONDITION, AES_ASP_TARGET, AES_EVENT_CONTRACT, AES_EXACT_ORDER,
    AES_HOST_AUTHORIZATION, AES_IDENTITY_PREPARATION, AES_LIMITS_CLAIM,
    AES_SCALAR_REPLACEMENT_APPLICATION, AES_SOURCE_BACKED_PREPARATION, AES_TRANSACTION_CONTRACT,
    AES_TRANSACTION_ENVELOPE, AES_TRANSACTION_INTEGRITY, AesSourceArtifact,
    AesTransactionApplication, AesTransactionAssertions, AesTransactionAuthorization,
    AesTransactionBody, AesTransactionEnvelope, AesTransactionEvidence,
    AesTransactionIntegrityPolicy, AesTransactionLimitsClaim, AesTransactionPrecondition,
    AesTransactionPreparation, AesTransactionSupport, AesTransactionTarget, IntegrityValue,
    PARTIAL_AES_PROFILE, TelexRecord, compute_aes_transaction_digest,
    encode_aes_transaction_signature_input, inspect_aes_transaction_envelope,
    inspect_aes_transaction_envelope_with_sources, validate_aes_transaction_body,
    validate_aes_transaction_envelope,
};
use serde_json::{Map, Value};

#[test]
fn passes_aes_transaction_candidate_vectors() -> Result<(), Box<dyn Error>> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/transactions/v0/aes-transaction-cts.v0.json");
    let manifest: Value = serde_json::from_str(&fs::read_to_string(&manifest_path)?)?;
    assert_eq!(
        manifest["meta"]["transaction_contract"],
        AES_TRANSACTION_CONTRACT
    );
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
    assert_eq!(count, 14);
    Ok(())
}

#[test]
fn supported_verified_transaction_still_requires_host_authorization() -> Result<(), Box<dyn Error>>
{
    let body = scalar_body();
    let hash = compute_aes_transaction_digest(&body, &[], &[])?.digest;
    let envelope = AesTransactionEnvelope {
        envelope: AES_TRANSACTION_ENVELOPE.to_owned(),
        body,
        evidence: Some(AesTransactionEvidence {
            integrity: AES_TRANSACTION_INTEGRITY.to_owned(),
            digest: "sha256".to_owned(),
            hash,
            signatures: Vec::new(),
        }),
    };
    let result = inspect_aes_transaction_envelope(&envelope, true, &scalar_support(), &[], &[]);
    assert!(result.ready_for_authorization);
    assert_eq!(result.provenance_verified, None);
    assert!(!result.actionable);
    Ok(())
}

#[test]
fn source_backed_preparation_requires_exact_retained_bytes() -> Result<(), Box<dyn Error>> {
    let origin = "sha256:d7f871f7c49226de258ccf11674f66e7395ab65f2e006e42851cd232b03c28e2";
    let mut body = scalar_body();
    body.application.contract = "x.example.source-check.v0".to_owned();
    body.preparation.contract = AES_SOURCE_BACKED_PREPARATION.to_owned();
    body.records = vec![TelexRecord::new(vec![
        ("path".to_owned(), "$.a".to_owned()),
        ("kind".to_owned(), "StringLiteral".to_owned()),
        ("value".to_owned(), "é".to_owned()),
        ("origin".to_owned(), origin.to_owned()),
        ("span".to_owned(), "4:8".to_owned()),
    ])];
    let hash = compute_aes_transaction_digest(&body, &[], &[])?.digest;
    let envelope = AesTransactionEnvelope {
        envelope: AES_TRANSACTION_ENVELOPE.to_owned(),
        body,
        evidence: Some(AesTransactionEvidence {
            integrity: AES_TRANSACTION_INTEGRITY.to_owned(),
            digest: "sha256".to_owned(),
            hash,
            signatures: Vec::new(),
        }),
    };
    let mut support = scalar_support();
    support.applications = vec!["x.example.source-check.v0".to_owned()];
    support.preparations = vec![AES_SOURCE_BACKED_PREPARATION.to_owned()];
    let unavailable = inspect_aes_transaction_envelope(&envelope, true, &support, &[], &[]);
    assert!(!unavailable.valid);
    assert_eq!(unavailable.provenance_verified, Some(false));
    assert!(!unavailable.ready_for_authorization);

    let verified = inspect_aes_transaction_envelope_with_sources(
        &envelope,
        true,
        &support,
        &[],
        &[],
        &[AesSourceArtifact {
            origin: origin.to_owned(),
            bytes: "a = \"é\"\r\n".as_bytes().to_vec(),
        }],
    );
    assert!(verified.valid);
    assert_eq!(verified.provenance_verified, Some(true));
    assert!(verified.ready_for_authorization);
    assert!(!verified.actionable);
    Ok(())
}

fn run_vector(vector: &Value) -> Result<(), Box<dyn Error>> {
    let id = vector["id"].as_str().ok_or("missing vector id")?;
    let operation = vector["operation"].as_str().ok_or("missing operation")?;
    let input = vector["input"].as_object().ok_or("missing input")?;
    let expected = vector["expected"].as_object().ok_or("missing expected")?;
    if operation == "signature-input" {
        let result = encode_aes_transaction_signature_input(
            required_string(input, "digest")?,
            required_string(input, "alg")?,
            required_string(input, "kid")?,
        )?;
        assert_eq!(
            lower_hex(&result.bytes),
            required_string(expected, "logical_hex")?,
            "{id}"
        );
        return Ok(());
    }

    let mut body = scalar_body();
    apply_patch(&mut body, input.get("patch"))?;
    let registered = input
        .get("registered_fields")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    if operation == "digest" {
        let result = compute_aes_transaction_digest(&body, &registered, &[])?;
        assert_eq!(result.digest, required_string(expected, "digest")?, "{id}");
        if let Some(logical_hex) = expected.get("logical_hex").and_then(Value::as_str) {
            assert_eq!(lower_hex(&result.bytes), logical_hex, "{id}");
        }
        return Ok(());
    }
    if operation == "validate-body" {
        let result = validate_aes_transaction_body(&body, &registered, &[]);
        assert_eq!(
            result.valid,
            expected["valid"].as_bool().ok_or("missing valid")?,
            "{id}"
        );
        assert_eq!(
            codes(&result.diagnostics),
            expected_codes(expected)?,
            "{id}"
        );
        return Ok(());
    }

    let evidence = evidence_for(input, &body, &registered)?;
    let envelope = AesTransactionEnvelope {
        envelope: AES_TRANSACTION_ENVELOPE.to_owned(),
        body,
        evidence,
    };
    let require_evidence = input
        .get("require_evidence")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if operation == "validate-envelope" {
        let result =
            validate_aes_transaction_envelope(&envelope, require_evidence, &registered, &[]);
        assert_eq!(
            result.valid,
            expected["valid"].as_bool().ok_or("missing valid")?,
            "{id}"
        );
        assert_eq!(
            result.evidence_verified,
            expected["evidence_verified"]
                .as_bool()
                .ok_or("missing evidence_verified")?,
            "{id}",
        );
        assert_eq!(
            codes(&result.diagnostics),
            expected_codes(expected)?,
            "{id}"
        );
        return Ok(());
    }
    if operation == "inspect-envelope" {
        let support = if input.get("support_ref").and_then(Value::as_str) == Some("scalar") {
            scalar_support()
        } else {
            AesTransactionSupport::default()
        };
        let result = inspect_aes_transaction_envelope(
            &envelope,
            require_evidence,
            &support,
            &registered,
            &[],
        );
        assert_eq!(
            result.valid,
            expected["valid"].as_bool().ok_or("missing valid")?,
            "{id}"
        );
        assert_eq!(
            result.supported,
            expected["supported"].as_bool().ok_or("missing supported")?,
            "{id}"
        );
        assert_eq!(
            result.evidence_verified,
            expected["evidence_verified"]
                .as_bool()
                .ok_or("missing evidence_verified")?,
            "{id}"
        );
        assert_eq!(
            result.ready_for_authorization,
            expected["ready_for_authorization"]
                .as_bool()
                .ok_or("missing ready_for_authorization")?,
            "{id}"
        );
        assert_eq!(
            result.actionable,
            expected["actionable"]
                .as_bool()
                .ok_or("missing actionable")?,
            "{id}"
        );
        assert_eq!(
            codes(&result.diagnostics),
            expected_codes(expected)?,
            "{id}"
        );
        return Ok(());
    }
    Err(format!("unsupported operation in {id}: {operation}").into())
}

fn scalar_body() -> AesTransactionBody {
    AesTransactionBody {
        transaction: AES_TRANSACTION_CONTRACT.to_owned(),
        id: "tx-1".to_owned(),
        intent: "intent-1".to_owned(),
        attempt: "attempt-1".to_owned(),
        events: AES_EVENT_CONTRACT.to_owned(),
        profile: PARTIAL_AES_PROFILE.to_owned(),
        projection: None,
        ordering: AES_EXACT_ORDER.to_owned(),
        application: AesTransactionApplication {
            contract: AES_SCALAR_REPLACEMENT_APPLICATION.to_owned(),
        },
        target: AesTransactionTarget {
            contract: AES_ASP_TARGET.to_owned(),
            id: "database-1".to_owned(),
            boundary: "$".to_owned(),
        },
        preconditions: vec![AesTransactionPrecondition {
            contract: AES_ASP_REVISION_PRECONDITION.to_owned(),
            scope: "$".to_owned(),
            revision: "7".to_owned(),
        }],
        preparation: AesTransactionPreparation {
            contract: AES_IDENTITY_PREPARATION.to_owned(),
        },
        authorization: AesTransactionAuthorization {
            contract: AES_HOST_AUTHORIZATION.to_owned(),
            context: "host-auth-1".to_owned(),
        },
        limits: AesTransactionLimitsClaim {
            contract: AES_LIMITS_CLAIM.to_owned(),
            id: "altopelago.aeonic-limits.v1".to_owned(),
            version: "1.0.0".to_owned(),
        },
        assertions: AesTransactionAssertions {
            event_count: "1".to_owned(),
            containers: Vec::new(),
        },
        records: vec![TelexRecord::new(vec![
            ("path".to_owned(), "$.status".to_owned()),
            ("kind".to_owned(), "StringLiteral".to_owned()),
            ("value".to_owned(), "ready".to_owned()),
        ])],
        integrity: AesTransactionIntegrityPolicy {
            contract: AES_TRANSACTION_INTEGRITY.to_owned(),
            digest: "sha256".to_owned(),
        },
        extensions: Vec::new(),
    }
}

fn scalar_support() -> AesTransactionSupport {
    AesTransactionSupport {
        applications: vec![AES_SCALAR_REPLACEMENT_APPLICATION.to_owned()],
        targets: vec![AES_ASP_TARGET.to_owned()],
        preconditions: vec![AES_ASP_REVISION_PRECONDITION.to_owned()],
        preparations: vec![AES_IDENTITY_PREPARATION.to_owned()],
        authorizations: vec![AES_HOST_AUTHORIZATION.to_owned()],
        limit_sets: vec!["altopelago.aeonic-limits.v1@1.0.0".to_owned()],
        integrity_contracts: vec![AES_TRANSACTION_INTEGRITY.to_owned()],
    }
}

fn apply_patch(body: &mut AesTransactionBody, patch: Option<&Value>) -> Result<(), Box<dyn Error>> {
    let Some(patch) = patch.and_then(Value::as_object) else {
        return Ok(());
    };
    for (path, value) in patch {
        match path.as_str() {
            "target.id" => {
                body.target.id = value.as_str().ok_or("target id must be string")?.to_owned()
            }
            "assertions.eventCount" => {
                body.assertions.event_count = value
                    .as_str()
                    .ok_or("event count must be string")?
                    .to_owned()
            }
            "attempt" => body.attempt = value.as_str().ok_or("attempt must be string")?.to_owned(),
            "x.example.note" => body
                .extensions
                .push((path.clone(), json_integrity_value(value)?)),
            "records.0.datatype" => {
                body.records[0] = TelexRecord::with_datatype(
                    body.records[0].fields().to_vec(),
                    aes_telex::DatatypeDescriptor {
                        datatype: value.as_str().ok_or("datatype must be string")?.to_owned(),
                        generics: Vec::new(),
                        clarifiers: Vec::new(),
                    },
                );
            }
            "records.0.generics" | "records.0.clarifiers" => {}
            _ => return Err(format!("unsupported test patch {path}").into()),
        }
    }
    Ok(())
}

fn evidence_for(
    input: &Map<String, Value>,
    body: &AesTransactionBody,
    registered: &[&str],
) -> Result<Option<AesTransactionEvidence>, Box<dyn Error>> {
    if input.contains_key("evidence") {
        return Ok(None);
    }
    let Some(reference) = input.get("evidence_ref").and_then(Value::as_str) else {
        return Ok(None);
    };
    let mut hash = compute_aes_transaction_digest(body, registered, &[])?.digest;
    if reference == "tampered" {
        let replacement = if hash.ends_with('0') { '1' } else { '0' };
        hash.pop();
        hash.push(replacement);
    }
    Ok(Some(AesTransactionEvidence {
        integrity: AES_TRANSACTION_INTEGRITY.to_owned(),
        digest: "sha256".to_owned(),
        hash,
        signatures: Vec::new(),
    }))
}

fn json_integrity_value(value: &Value) -> Result<IntegrityValue, Box<dyn Error>> {
    match value {
        Value::Null => Ok(IntegrityValue::Null),
        Value::String(value) => Ok(IntegrityValue::String(value.clone())),
        Value::Array(values) => Ok(IntegrityValue::List(
            values
                .iter()
                .map(json_integrity_value)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Value::Object(values) => Ok(IntegrityValue::Map(
            values
                .iter()
                .map(|(key, value)| Ok((key.clone(), json_integrity_value(value)?)))
                .collect::<Result<Vec<_>, Box<dyn Error>>>()?,
        )),
        _ => Err("integrity value must be null, string, list, or map".into()),
    }
}

fn expected_codes(expected: &Map<String, Value>) -> Result<Vec<String>, Box<dyn Error>> {
    let mut result = expected["diagnostic_codes"]
        .as_array()
        .ok_or("missing diagnostic codes")?
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or("diagnostic code must be string")
                .map(str::to_owned)
        })
        .collect::<Result<Vec<_>, _>>()?;
    result.sort();
    Ok(result)
}

fn codes(diagnostics: &[aes_telex::AesTransactionDiagnostic]) -> Vec<String> {
    let mut result = diagnostics
        .iter()
        .map(|item| item.code.to_owned())
        .collect::<Vec<_>>();
    result.sort();
    result.dedup();
    result
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
