//! Experimental Film v1 Candidate A codec.
//!
//! This module exercises the table-free layout recorded in the Film roadmap.
//! It is research code in an unpublished crate, not a released conformance
//! target. Comparator encodings must use a different preamble.

use std::error::Error;
use std::fmt;

use crate::{
    COMPLETE_AES_PROFILE, ClarifierKind, DatatypeClarifier, DatatypeDescriptor, Diagnostic,
    GenericArgument, ParsedTelex, TelexLimits, TelexRecord, encode_telex_with_projection,
    parse_telex, validate_telex_records_with_projection_and_limits,
};

pub const FILM_V1_PREAMBLE: [u8; 5] = [0x4f, 0x5f, 0x5f, 0xff, 0x01];

const CONTEXT_PROFILE: u8 = 0x01;
const CONTEXT_PROJECTION: u8 = 0x02;
const RECORD_HEADER: u8 = 0x01;
const RECORD_DATATYPE: u8 = 0x02;
const RECORD_IDENTITY: u8 = 0x04;
const RECORD_ORIGIN: u8 = 0x08;
const RECORD_SPAN: u8 = 0x10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilmLimits {
    pub max_input_bytes: usize,
    pub max_record_bytes: usize,
    pub max_field_bytes: usize,
    pub max_buffered_bytes: usize,
}

impl Default for FilmLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 67_108_864,
            max_record_bytes: 16_777_216,
            max_field_bytes: 4_194_304,
            max_buffered_bytes: 16_777_216,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilmStream {
    pub profile: String,
    pub profile_explicit: bool,
    pub projection: Option<String>,
    pub projection_explicit: bool,
    pub records: Vec<TelexRecord>,
}

impl From<&ParsedTelex> for FilmStream {
    fn from(parsed: &ParsedTelex) -> Self {
        Self {
            profile: parsed.profile.clone(),
            profile_explicit: parsed.profile_explicit,
            projection: parsed.projection.clone(),
            projection_explicit: parsed.projection_explicit,
            records: parsed.records.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilmError {
    pub code: &'static str,
    pub offset: usize,
    pub record: Option<usize>,
    pub component: &'static str,
    pub detail: String,
    pub diagnostics: Vec<Diagnostic>,
}

impl fmt::Display for FilmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.record {
            Some(record) => write!(
                formatter,
                "{} at byte {} in record {} ({}): {}",
                self.code, self.offset, record, self.component, self.detail
            ),
            None => write!(
                formatter,
                "{} at byte {} ({}): {}",
                self.code, self.offset, self.component, self.detail
            ),
        }
    }
}

impl Error for FilmError {}

#[must_use]
pub fn candidate_a_is_research_only() -> bool {
    true
}

pub fn encode_film_candidate_a(
    stream: &FilmStream,
    registered_fields: &[&str],
) -> Result<Vec<u8>, FilmError> {
    encode_film_candidate_a_with_limits(
        stream,
        registered_fields,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
}

pub fn encode_film_candidate_a_with_limits(
    stream: &FilmStream,
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<Vec<u8>, FilmError> {
    validate_context(stream)?;
    validate_stream(stream, registered_fields, aes_limits, 0)?;

    let mut output = Vec::new();
    output.extend_from_slice(&FILM_V1_PREAMBLE);
    let mut context = 0_u8;
    if stream.profile_explicit {
        context |= CONTEXT_PROFILE;
    }
    if stream.projection_explicit {
        context |= CONTEXT_PROJECTION;
    }
    output.push(context);
    if stream.profile_explicit {
        push_string(&mut output, &stream.profile, film_limits, "profile", None)?;
    }
    if stream.projection_explicit {
        push_string(
            &mut output,
            stream.projection.as_deref().unwrap_or_default(),
            film_limits,
            "projection",
            None,
        )?;
    }
    enforce_limit(
        "max_input_bytes",
        output.len(),
        film_limits.max_input_bytes,
        output.len(),
        None,
        "stream",
    )?;

    for (record_index, record) in stream.records.iter().enumerate() {
        let payload = encode_record(record, record_index, film_limits, aes_limits)?;
        enforce_limit(
            "max_record_bytes",
            payload.len(),
            film_limits.max_record_bytes,
            output.len(),
            Some(record_index),
            "record",
        )?;
        enforce_limit(
            "max_buffered_bytes",
            payload.len(),
            film_limits.max_buffered_bytes,
            output.len(),
            Some(record_index),
            "record",
        )?;
        let payload_length = usize_to_u64(payload.len(), output.len(), record_index)?;
        push_uleb(&mut output, payload_length);
        output.extend_from_slice(&payload);
        enforce_limit(
            "max_input_bytes",
            output.len(),
            film_limits.max_input_bytes,
            output.len(),
            Some(record_index),
            "stream",
        )?;
    }

    Ok(output)
}

pub fn decode_film_candidate_a(
    input: &[u8],
    registered_fields: &[&str],
) -> Result<FilmStream, FilmError> {
    decode_film_candidate_a_with_limits(
        input,
        registered_fields,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
}

pub fn decode_film_candidate_a_with_limits(
    input: &[u8],
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<FilmStream, FilmError> {
    enforce_limit(
        "max_input_bytes",
        input.len(),
        film_limits.max_input_bytes,
        0,
        None,
        "stream",
    )?;
    if input.len() < FILM_V1_PREAMBLE.len() {
        return Err(film_error(
            "FILM_TRUNCATED",
            input.len(),
            None,
            "preamble",
            "Film input ended inside the v1 preamble",
        ));
    }
    if input[..FILM_V1_PREAMBLE.len()] != FILM_V1_PREAMBLE {
        return Err(film_error(
            "FILM_INVALID_PREAMBLE",
            0,
            None,
            "preamble",
            "Expected O__ FF 01",
        ));
    }

    let mut reader = Reader::new(input, 0, None);
    reader.position = FILM_V1_PREAMBLE.len();
    let context = reader.read_byte("stream-context")?;
    if context & !0x03 != 0 {
        return Err(film_error(
            "FILM_NONCANONICAL",
            reader.absolute_position().saturating_sub(1),
            None,
            "stream-context",
            "Reserved stream-context bits must be zero",
        ));
    }
    let profile_explicit = context & CONTEXT_PROFILE != 0;
    let projection_explicit = context & CONTEXT_PROJECTION != 0;
    let profile = if profile_explicit {
        reader.read_nonempty_string(film_limits, "profile")?
    } else {
        COMPLETE_AES_PROFILE.to_owned()
    };
    let projection = if projection_explicit {
        Some(reader.read_nonempty_string(film_limits, "projection")?)
    } else {
        None
    };

    let mut records = Vec::new();
    while !reader.is_empty() {
        let record_index = records.len();
        if record_index >= aes_limits.max_events {
            return Err(limit_error(
                "max_events",
                record_index.saturating_add(1),
                aes_limits.max_events,
                reader.absolute_position(),
                Some(record_index),
                "record",
            ));
        }
        reader.record = Some(record_index);
        let payload_length = reader.read_length(film_limits, "record-length")?;
        if payload_length == 0 {
            return Err(film_error(
                "FILM_NONCANONICAL",
                reader.absolute_position(),
                Some(record_index),
                "record-length",
                "Film record payload length must be positive",
            ));
        }
        enforce_limit(
            "max_record_bytes",
            payload_length,
            film_limits.max_record_bytes,
            reader.absolute_position(),
            Some(record_index),
            "record",
        )?;
        let payload_offset = reader.absolute_position();
        let payload = reader.read_exact(payload_length, "record")?;
        records.push(decode_record(
            payload,
            payload_offset,
            record_index,
            film_limits,
            aes_limits,
        )?);
    }

    let stream = FilmStream {
        profile,
        profile_explicit,
        projection,
        projection_explicit,
        records,
    };
    validate_stream(&stream, registered_fields, aes_limits, input.len())?;
    Ok(stream)
}

pub fn telex_to_film_candidate_a(
    telex: &str,
    registered_fields: &[&str],
) -> Result<Vec<u8>, FilmError> {
    let parsed = parse_telex(telex)
        .map_err(|error| film_error(error.code, 0, None, "telex-source", error.detail))?;
    if !parsed.canonical {
        return Err(film_error(
            "FILM_SOURCE_NONCANONICAL",
            0,
            None,
            "telex-source",
            "Telex-to-Film transcoding requires canonical Telex input",
        ));
    }
    encode_film_candidate_a(&FilmStream::from(&parsed), registered_fields)
}

pub fn film_candidate_a_to_telex(
    film: &[u8],
    registered_fields: &[&str],
) -> Result<String, FilmError> {
    let stream = decode_film_candidate_a(film, registered_fields)?;
    let profile = stream.profile_explicit.then_some(stream.profile.as_str());
    let projection = stream
        .projection_explicit
        .then_some(stream.projection.as_deref())
        .flatten();
    encode_telex_with_projection(&stream.records, profile, projection)
        .map_err(|error| film_error(error.code, 0, None, "telex-target", error.detail))
}

fn validate_context(stream: &FilmStream) -> Result<(), FilmError> {
    if stream.profile.is_empty() {
        return Err(film_error(
            "FILM_ENCODE_ERROR",
            0,
            None,
            "profile",
            "Film profile must not be empty",
        ));
    }
    if !stream.profile_explicit && stream.profile != COMPLETE_AES_PROFILE {
        return Err(film_error(
            "FILM_ENCODE_ERROR",
            0,
            None,
            "profile",
            "An omitted profile must have the aes.complete.v1 effective value",
        ));
    }
    match (stream.projection_explicit, stream.projection.as_deref()) {
        (false, None) => Ok(()),
        (true, Some(value)) if !value.is_empty() => Ok(()),
        (false, Some(_)) => Err(film_error(
            "FILM_ENCODE_ERROR",
            0,
            None,
            "projection",
            "An omitted projection cannot carry a value",
        )),
        (true, _) => Err(film_error(
            "FILM_ENCODE_ERROR",
            0,
            None,
            "projection",
            "An explicit projection must have a non-empty value",
        )),
    }
}

fn validate_stream(
    stream: &FilmStream,
    registered_fields: &[&str],
    limits: &TelexLimits,
    offset: usize,
) -> Result<(), FilmError> {
    let validation = validate_telex_records_with_projection_and_limits(
        &stream.records,
        &stream.profile,
        stream.projection.as_deref(),
        registered_fields,
        limits,
    );
    if validation.valid {
        return Ok(());
    }
    let record = validation.diagnostics.first().and_then(|item| item.record);
    let detail = validation.diagnostics.first().map_or_else(
        || "Portable AES validation failed".to_owned(),
        |item| item.message.clone(),
    );
    Err(FilmError {
        code: "FILM_AES_INVALID",
        offset,
        record,
        component: "aes-events",
        detail,
        diagnostics: validation.diagnostics,
    })
}

fn encode_record(
    record: &TelexRecord,
    record_index: usize,
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<Vec<u8>, FilmError> {
    reject_duplicate_fields(record, record_index)?;
    let (address_name, address) = match (record.get("path"), record.get("header")) {
        (Some(path), None) => ("path", path),
        (None, Some(header)) => ("header", header),
        _ => {
            return Err(film_error(
                "FILM_ENCODE_ERROR",
                0,
                Some(record_index),
                "address",
                "Film records require exactly one path or header",
            ));
        }
    };
    let kind = record.get("kind").ok_or_else(|| {
        film_error(
            "FILM_ENCODE_ERROR",
            0,
            Some(record_index),
            "kind",
            "Film records require a kind",
        )
    })?;
    let kind_code = kind_code(kind).ok_or_else(|| {
        film_error(
            "FILM_ENCODE_ERROR",
            0,
            Some(record_index),
            "kind",
            format!("Unassigned Film v1 kind: {kind}"),
        )
    })?;
    let datatype = record.datatype();
    let identity = record.get("identity");
    let origin = record.get("origin");
    let span = record.get("span");
    let mut control = 0_u8;
    if address_name == "header" {
        control |= RECORD_HEADER;
    }
    if datatype.is_some() {
        control |= RECORD_DATATYPE;
    }
    if identity.is_some() {
        control |= RECORD_IDENTITY;
    }
    if origin.is_some() {
        control |= RECORD_ORIGIN;
    }
    if span.is_some() {
        control |= RECORD_SPAN;
    }

    let mut output = vec![control, kind_code];
    push_string(
        &mut output,
        address,
        film_limits,
        "address",
        Some(record_index),
    )?;
    if let Some(descriptor) = datatype {
        push_descriptor(
            &mut output,
            descriptor,
            0,
            film_limits,
            aes_limits,
            record_index,
        )?;
    }
    if let Some(identity) = identity {
        push_string(
            &mut output,
            identity,
            film_limits,
            "identity",
            Some(record_index),
        )?;
    }
    if kind_has_value(kind) {
        let value = record.get("value").ok_or_else(|| {
            film_error(
                "FILM_ENCODE_ERROR",
                0,
                Some(record_index),
                "value",
                format!("Kind {kind} requires a value"),
            )
        })?;
        push_string(&mut output, value, film_limits, "value", Some(record_index))?;
    }
    if let Some(origin) = origin {
        output.extend_from_slice(&decode_origin(origin, record_index)?);
    }
    if let Some(span) = span {
        let (start, end) = decode_span(span, record_index)?;
        push_uleb(&mut output, start);
        push_uleb(&mut output, end);
    }

    let mut extensions = record
        .fields()
        .iter()
        .filter(|(name, _)| !is_core_field(name))
        .collect::<Vec<_>>();
    extensions.sort_by(|left, right| left.0.cmp(&right.0));
    for (name, value) in extensions {
        if !valid_extension_name(name) {
            return Err(film_error(
                "FILM_ENCODE_ERROR",
                0,
                Some(record_index),
                "extension-name",
                format!("Invalid Film extension name: {name}"),
            ));
        }
        push_string(
            &mut output,
            name,
            film_limits,
            "extension-name",
            Some(record_index),
        )?;
        push_string(
            &mut output,
            value,
            film_limits,
            "extension-value",
            Some(record_index),
        )?;
    }
    Ok(output)
}

fn decode_record(
    payload: &[u8],
    payload_offset: usize,
    record_index: usize,
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<TelexRecord, FilmError> {
    let mut reader = Reader::new(payload, payload_offset, Some(record_index));
    let control = reader.read_byte("record-control")?;
    if control & !0x1f != 0 {
        return Err(film_error(
            "FILM_NONCANONICAL",
            payload_offset,
            Some(record_index),
            "record-control",
            "Reserved record-control bits must be zero",
        ));
    }
    if control & RECORD_SPAN != 0 && control & RECORD_ORIGIN == 0 {
        return Err(film_error(
            "FILM_INVALID_RECORD",
            payload_offset,
            Some(record_index),
            "record-control",
            "Span presence requires origin presence",
        ));
    }
    let kind_offset = reader.absolute_position();
    let kind = kind_name(reader.read_byte("kind")?).ok_or_else(|| {
        film_error(
            "FILM_INVALID_KIND",
            kind_offset,
            Some(record_index),
            "kind",
            "Unassigned Film v1 kind code",
        )
    })?;
    let address = reader.read_string(film_limits, "address")?;
    let address_name = if control & RECORD_HEADER != 0 {
        "header"
    } else {
        "path"
    };
    let mut fields = vec![
        (address_name.to_owned(), address),
        ("kind".to_owned(), kind.to_owned()),
    ];
    let datatype = if control & RECORD_DATATYPE != 0 {
        let descriptor = reader.read_descriptor(film_limits, aes_limits, 0)?;
        fields.push(("datatype".to_owned(), descriptor.datatype.clone()));
        Some(descriptor)
    } else {
        None
    };
    if control & RECORD_IDENTITY != 0 {
        fields.push((
            "identity".to_owned(),
            reader.read_string(film_limits, "identity")?,
        ));
    }
    if kind_has_value(kind) {
        fields.push((
            "value".to_owned(),
            reader.read_string(film_limits, "value")?,
        ));
    }
    if control & RECORD_ORIGIN != 0 {
        let bytes = reader.read_exact(32, "origin")?;
        fields.push(("origin".to_owned(), encode_origin(bytes)));
    }
    if control & RECORD_SPAN != 0 {
        let start = reader.read_uleb("span-start")?;
        let end = reader.read_uleb("span-end")?;
        if start >= end {
            return Err(film_error(
                "FILM_INVALID_RECORD",
                reader.absolute_position(),
                Some(record_index),
                "span",
                "Film span requires start < end",
            ));
        }
        fields.push(("span".to_owned(), format!("{start}:{end}")));
    }

    let mut previous_extension: Option<String> = None;
    while !reader.is_empty() {
        let name_offset = reader.absolute_position();
        let name = reader.read_nonempty_string(film_limits, "extension-name")?;
        if !valid_extension_name(&name) {
            return Err(film_error(
                "FILM_INVALID_RECORD",
                name_offset,
                Some(record_index),
                "extension-name",
                format!("Invalid Film extension name: {name}"),
            ));
        }
        if previous_extension
            .as_ref()
            .is_some_and(|previous| previous >= &name)
        {
            return Err(film_error(
                "FILM_NONCANONICAL",
                name_offset,
                Some(record_index),
                "extension-name",
                "Extensions must be strictly ordered without duplicates",
            ));
        }
        let value = reader.read_string(film_limits, "extension-value")?;
        previous_extension = Some(name.clone());
        fields.push((name, value));
    }

    Ok(match datatype {
        Some(descriptor) => TelexRecord::with_datatype(fields, descriptor),
        None => TelexRecord::new(fields),
    })
}

fn push_descriptor(
    output: &mut Vec<u8>,
    descriptor: &DatatypeDescriptor,
    depth: usize,
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
    record_index: usize,
) -> Result<(), FilmError> {
    if depth > aes_limits.max_generic_depth {
        return Err(film_error(
            "FILM_ENCODE_ERROR",
            output.len(),
            Some(record_index),
            "datatype",
            "Datatype generic depth exceeds the active AES limit",
        ));
    }
    let mut body = Vec::new();
    push_string(
        &mut body,
        &descriptor.datatype,
        film_limits,
        "datatype-name",
        Some(record_index),
    )?;
    push_uleb(
        &mut body,
        usize_to_u64(descriptor.generics.len(), output.len(), record_index)?,
    );
    for generic in &descriptor.generics {
        match generic {
            GenericArgument::Datatype(nested) => {
                body.push(0x00);
                push_descriptor(
                    &mut body,
                    nested,
                    depth.saturating_add(1),
                    film_limits,
                    aes_limits,
                    record_index,
                )?;
            }
            GenericArgument::NumberLiteral(value) => {
                body.push(0x02);
                push_string(
                    &mut body,
                    value,
                    film_limits,
                    "generic-number",
                    Some(record_index),
                )?;
            }
        }
    }
    push_uleb(
        &mut body,
        usize_to_u64(descriptor.clarifiers.len(), output.len(), record_index)?,
    );
    for clarifier in &descriptor.clarifiers {
        body.push(match clarifier.kind {
            ClarifierKind::StringLiteral => 0x01,
            ClarifierKind::NumberLiteral => 0x02,
        });
        push_string(
            &mut body,
            &clarifier.value,
            film_limits,
            "clarifier-value",
            Some(record_index),
        )?;
    }
    enforce_limit(
        "max_field_bytes",
        body.len(),
        film_limits.max_field_bytes,
        output.len(),
        Some(record_index),
        "datatype",
    )?;
    push_uleb(
        output,
        usize_to_u64(body.len(), output.len(), record_index)?,
    );
    output.extend_from_slice(&body);
    Ok(())
}

fn reject_duplicate_fields(record: &TelexRecord, record_index: usize) -> Result<(), FilmError> {
    for (index, (name, _)) in record.fields().iter().enumerate() {
        if record.fields()[..index]
            .iter()
            .any(|(existing, _)| existing == name)
        {
            return Err(film_error(
                "FILM_ENCODE_ERROR",
                0,
                Some(record_index),
                "record",
                format!("Duplicate AES field: {name}"),
            ));
        }
    }
    Ok(())
}

fn push_string(
    output: &mut Vec<u8>,
    value: &str,
    limits: &FilmLimits,
    component: &'static str,
    record: Option<usize>,
) -> Result<(), FilmError> {
    enforce_limit(
        "max_field_bytes",
        value.len(),
        limits.max_field_bytes,
        output.len(),
        record,
        component,
    )?;
    let length = u64::try_from(value.len()).map_err(|_| {
        film_error(
            "FILM_ENCODE_ERROR",
            output.len(),
            record,
            component,
            "String length does not fit the Film u64 domain",
        )
    })?;
    push_uleb(output, length);
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn push_uleb(output: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn kind_code(kind: &str) -> Option<u8> {
    KIND_NAMES
        .iter()
        .position(|candidate| *candidate == kind)
        .and_then(|index| u8::try_from(index.saturating_add(1)).ok())
}

fn kind_name(code: u8) -> Option<&'static str> {
    code.checked_sub(1)
        .and_then(|index| KIND_NAMES.get(usize::from(index)))
        .copied()
}

fn kind_has_value(kind: &str) -> bool {
    !["ObjectNode", "ListNode", "TupleLiteral", "NodeLiteral"].contains(&kind)
}

fn is_core_field(name: &str) -> bool {
    [
        "path", "header", "kind", "datatype", "identity", "value", "origin", "span",
    ]
    .contains(&name)
}

fn valid_extension_name(name: &str) -> bool {
    let segments = name.split('.').collect::<Vec<_>>();
    segments.len() >= 3
        && segments[0] == "x"
        && segments[1..].iter().all(|segment| {
            let mut bytes = segment.bytes();
            bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
                && bytes
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn decode_origin(origin: &str, record_index: usize) -> Result<[u8; 32], FilmError> {
    let digest = origin.strip_prefix("sha256:").ok_or_else(|| {
        film_error(
            "FILM_ENCODE_ERROR",
            0,
            Some(record_index),
            "origin",
            "Origin must use sha256",
        )
    })?;
    if digest.len() != 64 {
        return Err(film_error(
            "FILM_ENCODE_ERROR",
            0,
            Some(record_index),
            "origin",
            "SHA-256 origin must contain 64 lowercase hexadecimal digits",
        ));
    }
    let mut bytes = [0_u8; 32];
    for (index, pair) in digest.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(pair[0]).ok_or_else(|| {
            film_error(
                "FILM_ENCODE_ERROR",
                0,
                Some(record_index),
                "origin",
                "SHA-256 origin must be lowercase hexadecimal",
            )
        })?;
        let low = hex_nibble(pair[1]).ok_or_else(|| {
            film_error(
                "FILM_ENCODE_ERROR",
                0,
                Some(record_index),
                "origin",
                "SHA-256 origin must be lowercase hexadecimal",
            )
        })?;
        bytes[index] = (high << 4) | low;
    }
    Ok(bytes)
}

fn encode_origin(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(71);
    output.push_str("sha256:");
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn decode_span(span: &str, record_index: usize) -> Result<(u64, u64), FilmError> {
    let (start, end) = span.split_once(':').ok_or_else(|| {
        film_error(
            "FILM_ENCODE_ERROR",
            0,
            Some(record_index),
            "span",
            "Span must contain start:end",
        )
    })?;
    let start = start.parse::<u64>().map_err(|error| {
        film_error(
            "FILM_ENCODE_ERROR",
            0,
            Some(record_index),
            "span",
            error.to_string(),
        )
    })?;
    let end = end.parse::<u64>().map_err(|error| {
        film_error(
            "FILM_ENCODE_ERROR",
            0,
            Some(record_index),
            "span",
            error.to_string(),
        )
    })?;
    if start >= end {
        return Err(film_error(
            "FILM_ENCODE_ERROR",
            0,
            Some(record_index),
            "span",
            "Span requires start < end",
        ));
    }
    Ok((start, end))
}

fn usize_to_u64(value: usize, offset: usize, record_index: usize) -> Result<u64, FilmError> {
    u64::try_from(value).map_err(|_| {
        film_error(
            "FILM_ENCODE_ERROR",
            offset,
            Some(record_index),
            "length",
            "Length does not fit the Film u64 domain",
        )
    })
}

fn film_error(
    code: &'static str,
    offset: usize,
    record: Option<usize>,
    component: &'static str,
    detail: impl Into<String>,
) -> FilmError {
    FilmError {
        code,
        offset,
        record,
        component,
        detail: detail.into(),
        diagnostics: Vec::new(),
    }
}

fn limit_error(
    counter: &'static str,
    observed: usize,
    limit: usize,
    offset: usize,
    record: Option<usize>,
    component: &'static str,
) -> FilmError {
    film_error(
        "FILM_LIMIT_EXCEEDED",
        offset,
        record,
        component,
        format!("{counter} observed {observed}, limit {limit}"),
    )
}

fn enforce_limit(
    counter: &'static str,
    observed: usize,
    limit: usize,
    offset: usize,
    record: Option<usize>,
    component: &'static str,
) -> Result<(), FilmError> {
    if observed > limit {
        Err(limit_error(
            counter, observed, limit, offset, record, component,
        ))
    } else {
        Ok(())
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
    base_offset: usize,
    record: Option<usize>,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], base_offset: usize, record: Option<usize>) -> Self {
        Self {
            bytes,
            position: 0,
            base_offset,
            record,
        }
    }

    fn is_empty(&self) -> bool {
        self.position == self.bytes.len()
    }

    fn absolute_position(&self) -> usize {
        self.base_offset.saturating_add(self.position)
    }

    fn read_byte(&mut self, component: &'static str) -> Result<u8, FilmError> {
        let byte = self.bytes.get(self.position).copied().ok_or_else(|| {
            film_error(
                "FILM_TRUNCATED",
                self.absolute_position(),
                self.record,
                component,
                "Film input ended before the required byte",
            )
        })?;
        self.position = self.position.saturating_add(1);
        Ok(byte)
    }

    fn read_exact(
        &mut self,
        length: usize,
        component: &'static str,
    ) -> Result<&'a [u8], FilmError> {
        let end = self.position.checked_add(length).ok_or_else(|| {
            film_error(
                "FILM_INTEGER_OVERFLOW",
                self.absolute_position(),
                self.record,
                component,
                "Byte range overflow",
            )
        })?;
        let bytes = self.bytes.get(self.position..end).ok_or_else(|| {
            film_error(
                "FILM_TRUNCATED",
                self.absolute_position(),
                self.record,
                component,
                "Film input ended inside a declared field or record",
            )
        })?;
        self.position = end;
        Ok(bytes)
    }

    fn read_uleb(&mut self, component: &'static str) -> Result<u64, FilmError> {
        let start = self.absolute_position();
        let mut value = 0_u64;
        for index in 0..10_u32 {
            let byte = self.read_byte(component)?;
            let payload = u64::from(byte & 0x7f);
            if index == 9 && payload > 1 {
                return Err(film_error(
                    "FILM_INTEGER_OVERFLOW",
                    start,
                    self.record,
                    component,
                    "Unsigned LEB128 value exceeds u64",
                ));
            }
            value |= payload << (index * 7);
            if byte & 0x80 == 0 {
                let width = usize::try_from(index + 1).unwrap_or(10);
                if uleb_width(value) != width {
                    return Err(film_error(
                        "FILM_NONCANONICAL",
                        start,
                        self.record,
                        component,
                        "Unsigned LEB128 must use its shortest representation",
                    ));
                }
                return Ok(value);
            }
        }
        Err(film_error(
            "FILM_INTEGER_OVERFLOW",
            start,
            self.record,
            component,
            "Unsigned LEB128 exceeds ten bytes",
        ))
    }

    fn read_length(
        &mut self,
        limits: &FilmLimits,
        component: &'static str,
    ) -> Result<usize, FilmError> {
        let offset = self.absolute_position();
        let value = self.read_uleb(component)?;
        let length = usize::try_from(value).map_err(|_| {
            film_error(
                "FILM_INTEGER_OVERFLOW",
                offset,
                self.record,
                component,
                "Film length does not fit the host address space",
            )
        })?;
        if component != "record-length" {
            enforce_limit(
                "max_field_bytes",
                length,
                limits.max_field_bytes,
                offset,
                self.record,
                component,
            )?;
        }
        Ok(length)
    }

    fn read_string(
        &mut self,
        limits: &FilmLimits,
        component: &'static str,
    ) -> Result<String, FilmError> {
        let offset = self.absolute_position();
        let length = self.read_length(limits, component)?;
        let bytes = self.read_exact(length, component)?;
        let value = std::str::from_utf8(bytes).map_err(|error| {
            film_error(
                "FILM_INVALID_UTF8",
                offset.saturating_add(error.valid_up_to()),
                self.record,
                component,
                "Film strings must contain valid UTF-8",
            )
        })?;
        Ok(value.to_owned())
    }

    fn read_nonempty_string(
        &mut self,
        limits: &FilmLimits,
        component: &'static str,
    ) -> Result<String, FilmError> {
        let offset = self.absolute_position();
        let value = self.read_string(limits, component)?;
        if value.is_empty() {
            return Err(film_error(
                "FILM_INVALID_RECORD",
                offset,
                self.record,
                component,
                "This Film string must not be empty",
            ));
        }
        Ok(value)
    }

    fn read_descriptor(
        &mut self,
        film_limits: &FilmLimits,
        aes_limits: &TelexLimits,
        depth: usize,
    ) -> Result<DatatypeDescriptor, FilmError> {
        if depth > aes_limits.max_generic_depth {
            return Err(film_error(
                "FILM_LIMIT_EXCEEDED",
                self.absolute_position(),
                self.record,
                "datatype",
                "Datatype generic depth exceeds the active AES limit",
            ));
        }
        let length = self.read_length(film_limits, "datatype")?;
        let offset = self.absolute_position();
        let bytes = self.read_exact(length, "datatype")?;
        let mut descriptor = Reader::new(bytes, offset, self.record);
        let datatype = descriptor.read_nonempty_string(film_limits, "datatype-name")?;
        let generic_count = descriptor.read_length(film_limits, "generic-count")?;
        if generic_count > aes_limits.max_generic_arguments {
            return Err(limit_error(
                "max_generic_arguments",
                generic_count,
                aes_limits.max_generic_arguments,
                descriptor.absolute_position(),
                self.record,
                "generic-count",
            ));
        }
        let mut generics = Vec::with_capacity(generic_count);
        for _ in 0..generic_count {
            let tag_offset = descriptor.absolute_position();
            match descriptor.read_byte("generic-tag")? {
                0x00 => generics.push(GenericArgument::Datatype(descriptor.read_descriptor(
                    film_limits,
                    aes_limits,
                    depth.saturating_add(1),
                )?)),
                0x02 => generics.push(GenericArgument::NumberLiteral(
                    descriptor.read_string(film_limits, "generic-number")?,
                )),
                _ => {
                    return Err(film_error(
                        "FILM_INVALID_TAG",
                        tag_offset,
                        self.record,
                        "generic-tag",
                        "Film generic tag must be 00 or 02",
                    ));
                }
            }
        }
        let clarifier_count = descriptor.read_length(film_limits, "clarifier-count")?;
        if clarifier_count > aes_limits.max_clarifier_values {
            return Err(limit_error(
                "max_clarifier_values",
                clarifier_count,
                aes_limits.max_clarifier_values,
                descriptor.absolute_position(),
                self.record,
                "clarifier-count",
            ));
        }
        let mut clarifiers = Vec::with_capacity(clarifier_count);
        for _ in 0..clarifier_count {
            let tag_offset = descriptor.absolute_position();
            let kind = match descriptor.read_byte("clarifier-tag")? {
                0x01 => ClarifierKind::StringLiteral,
                0x02 => ClarifierKind::NumberLiteral,
                _ => {
                    return Err(film_error(
                        "FILM_INVALID_TAG",
                        tag_offset,
                        self.record,
                        "clarifier-tag",
                        "Film clarifier tag must be 01 or 02",
                    ));
                }
            };
            clarifiers.push(DatatypeClarifier {
                kind,
                value: descriptor.read_string(film_limits, "clarifier-value")?,
            });
        }
        if !descriptor.is_empty() {
            return Err(film_error(
                "FILM_NONCANONICAL",
                descriptor.absolute_position(),
                self.record,
                "datatype",
                "Datatype descriptor length was not consumed exactly",
            ));
        }
        Ok(DatatypeDescriptor {
            datatype,
            generics,
            clarifiers,
        })
    }
}

fn uleb_width(mut value: u64) -> usize {
    let mut width = 1_usize;
    while value >= 0x80 {
        value >>= 7;
        width = width.saturating_add(1);
    }
    width
}

const KIND_NAMES: [&str; 23] = [
    "StringLiteral",
    "NumberLiteral",
    "InfinityLiteral",
    "NaNLiteral",
    "NullLiteral",
    "BooleanLiteral",
    "ToggleLiteral",
    "HexLiteral",
    "RadixLiteral",
    "EncodingLiteral",
    "SeparatorLiteral",
    "SansaAddressLiteral",
    "DateLiteral",
    "TimeLiteral",
    "DateTimeLiteral",
    "WTCDateTimeLiteral",
    "ObjectNode",
    "ListNode",
    "TupleLiteral",
    "NodeLiteral",
    "NodeHead",
    "CloneReference",
    "PointerReference",
];
