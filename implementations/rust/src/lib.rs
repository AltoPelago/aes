use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;

const VERSION_LINE: &str = "telex.aes=0";
const CORE_FIELDS: [&str; 8] = [
    "header", "path", "kind", "datatype", "identity", "value", "origin", "span",
];
const VALUE_KINDS: [&str; 23] = [
    "string",
    "number",
    "infinity",
    "nan",
    "null",
    "boolean",
    "toggle",
    "hex",
    "radix",
    "encoding",
    "separator",
    "sansa-address",
    "date",
    "time",
    "datetime",
    "wtc",
    "object",
    "list",
    "tuple",
    "node",
    "node-head",
    "clone-reference",
    "pointer-reference",
];

pub const TELEX_VERSION: &str = "0";
pub const COMPLETE_AES_PROFILE: &str = "aes.complete.v0";
pub const PARTIAL_AES_PROFILE: &str = "aes.partial.v0";
pub const AEON_DOCUMENT_PROJECTION: &str = "aeon.document.v0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelexRecord {
    fields: Vec<(String, String)>,
}

impl TelexRecord {
    #[must_use]
    pub fn new(fields: Vec<(String, String)>) -> Self {
        Self { fields }
    }

    #[must_use]
    pub fn fields(&self) -> &[(String, String)] {
        &self.fields
    }

    #[must_use]
    pub fn get(&self, field: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, value)| value.as_str())
    }

    #[must_use]
    pub fn contains(&self, field: &str) -> bool {
        self.get(field).is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTelex {
    pub version: String,
    pub profile: String,
    pub profile_explicit: bool,
    pub projection: Option<String>,
    pub projection_explicit: bool,
    pub records: Vec<TelexRecord>,
    pub canonical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelexSyntaxError {
    pub code: &'static str,
    pub line: Option<usize>,
    pub detail: String,
}

impl TelexSyntaxError {
    fn new(code: &'static str, detail: impl Into<String>, line: Option<usize>) -> Self {
        Self {
            code,
            line,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for TelexSyntaxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(formatter, "Line {line}: {}", self.detail),
            None => formatter.write_str(&self.detail),
        }
    }
}

impl Error for TelexSyntaxError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelexEncodeError {
    pub detail: String,
}

impl fmt::Display for TelexEncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl Error for TelexEncodeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
    pub record: Option<usize>,
    pub path: Option<String>,
    pub field: Option<String>,
    pub first_record: Option<usize>,
    pub required_path: Option<String>,
}

impl Diagnostic {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            record: None,
            path: None,
            field: None,
            first_record: None,
            required_path: None,
        }
    }

    fn at_record(mut self, record: usize, path: Option<&str>) -> Self {
        self.record = Some(record);
        self.path = path.map(str::to_owned);
        self
    }

    fn with_field(mut self, field: &str) -> Self {
        self.field = Some(field.to_owned());
        self
    }

    fn with_first_record(mut self, record: usize) -> Self {
        self.first_record = Some(record);
        self
    }

    fn with_required_path(mut self, path: &str) -> Self {
        self.required_path = Some(path.to_owned());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationResult {
    pub valid: bool,
    pub profile: String,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse_telex(input: &str) -> Result<ParsedTelex, TelexSyntaxError> {
    if input.starts_with('\u{feff}') {
        return Err(TelexSyntaxError::new(
            "TELEX_BOM",
            "UTF-8 byte-order marks are not allowed",
            Some(1),
        ));
    }
    if input
        .as_bytes()
        .iter()
        .enumerate()
        .any(|(index, byte)| *byte == b'\r' && input.as_bytes().get(index + 1) != Some(&b'\n'))
    {
        return Err(TelexSyntaxError::new(
            "TELEX_BARE_CR",
            "Bare carriage returns are not allowed",
            None,
        ));
    }

    let canonical_line_endings = !input.contains("\r\n");
    let normalized = input.replace("\r\n", "\n");
    let has_final_lf = normalized.ends_with('\n');
    let mut lines: Vec<&str> = normalized.split('\n').collect();
    if has_final_lf {
        lines.pop();
    }

    if lines.first().copied() != Some(VERSION_LINE) {
        return Err(TelexSyntaxError::new(
            "TELEX_INVALID_PREAMBLE",
            format!("Expected {VERSION_LINE}"),
            Some(1),
        ));
    }

    if lines.len() == 1 {
        return Ok(ParsedTelex {
            version: TELEX_VERSION.to_owned(),
            profile: COMPLETE_AES_PROFILE.to_owned(),
            profile_explicit: false,
            projection: None,
            projection_explicit: false,
            records: Vec::new(),
            canonical: canonical_line_endings && has_final_lf,
        });
    }

    let mut profile = COMPLETE_AES_PROFILE.to_owned();
    let mut profile_explicit = false;
    let mut projection = None;
    let mut projection_explicit = false;
    let mut header_canonical = true;
    let mut event_start = 1_usize;
    let mut last_header_rank = None;
    while let Some(line) = lines.get(event_start).filter(|line| !line.is_empty()) {
        let Some((field, payload)) = line.split_once('=') else {
            break;
        };
        let rank = match field {
            "profile" => 0_usize,
            "projection" => 1_usize,
            _ => break,
        };
        if last_header_rank.is_some_and(|previous| rank < previous) {
            header_canonical = false;
        }
        last_header_rank = Some(rank);
        let line_number = event_start + 1;
        let decoded = decode_payload(payload, line_number)?;
        header_canonical &= decoded.canonical;
        if decoded.value.is_empty() {
            let (code, label) = if field == "profile" {
                ("TELEX_EMPTY_PROFILE", "Profile")
            } else {
                ("TELEX_EMPTY_PROJECTION", "Projection")
            };
            return Err(TelexSyntaxError::new(
                code,
                format!("{label} identifier must not be empty"),
                Some(line_number),
            ));
        }
        if field == "profile" {
            if profile_explicit {
                return Err(TelexSyntaxError::new(
                    "TELEX_DUPLICATE_STREAM_FIELD",
                    "Duplicate stream field: profile",
                    Some(line_number),
                ));
            }
            profile = decoded.value;
            profile_explicit = true;
        } else {
            if projection_explicit {
                return Err(TelexSyntaxError::new(
                    "TELEX_DUPLICATE_STREAM_FIELD",
                    "Duplicate stream field: projection",
                    Some(line_number),
                ));
            }
            projection = Some(decoded.value);
            projection_explicit = true;
        }
        event_start += 1;
    }

    if event_start == lines.len() {
        return Ok(ParsedTelex {
            version: TELEX_VERSION.to_owned(),
            profile,
            profile_explicit,
            projection,
            projection_explicit,
            records: Vec::new(),
            canonical: canonical_line_endings && has_final_lf && header_canonical,
        });
    }

    if lines.get(event_start).copied() != Some("") {
        return Err(TelexSyntaxError::new(
            "TELEX_MISSING_HEADER_SEPARATOR",
            "Expected a blank line after the stream header",
            Some(event_start + 1),
        ));
    }
    event_start += 1;

    let mut records = Vec::new();
    let mut record: Option<Vec<(String, String)>> = None;
    let mut canonical = canonical_line_endings && has_final_lf && header_canonical;
    let mut separator_width = 1_usize;

    for (index, line) in lines.iter().enumerate().skip(event_start) {
        let line_number = index + 1;
        if line.is_empty() {
            separator_width += 1;
            if let Some(fields) = record.take() {
                records.push(TelexRecord::new(fields));
            }
            continue;
        }

        if separator_width > 1 {
            canonical = false;
        }
        separator_width = 0;
        let fields = record.get_or_insert_with(Vec::new);
        let Some(delimiter) = line.find('=') else {
            return Err(TelexSyntaxError::new(
                "TELEX_INVALID_FIELD_LINE",
                "Expected field=value",
                Some(line_number),
            ));
        };
        if delimiter == 0 {
            return Err(TelexSyntaxError::new(
                "TELEX_INVALID_FIELD_LINE",
                "Expected field=value",
                Some(line_number),
            ));
        }
        let field = &line[..delimiter];
        if !valid_field_name(field) {
            return Err(TelexSyntaxError::new(
                "TELEX_INVALID_FIELD_NAME",
                format!("Invalid field name: {field}"),
                Some(line_number),
            ));
        }
        if fields.iter().any(|(existing, _)| existing == field) {
            return Err(TelexSyntaxError::new(
                "TELEX_DUPLICATE_FIELD",
                format!("Duplicate field: {field}"),
                Some(line_number),
            ));
        }
        let decoded = decode_payload(&line[delimiter + 1..], line_number)?;
        canonical &= decoded.canonical;
        fields.push((field.to_owned(), decoded.value));
    }

    if let Some(fields) = record {
        records.push(TelexRecord::new(fields));
    }
    if separator_width > 0 {
        canonical = false;
    }
    canonical &= records.iter().all(has_canonical_field_order);

    Ok(ParsedTelex {
        version: TELEX_VERSION.to_owned(),
        profile,
        profile_explicit,
        projection,
        projection_explicit,
        records,
        canonical,
    })
}

pub fn encode_telex(
    records: &[TelexRecord],
    profile: Option<&str>,
) -> Result<String, TelexEncodeError> {
    encode_telex_with_projection(records, profile, None)
}

pub fn encode_telex_with_projection(
    records: &[TelexRecord],
    profile: Option<&str>,
    projection: Option<&str>,
) -> Result<String, TelexEncodeError> {
    if profile == Some("") {
        return Err(TelexEncodeError {
            detail: "Telex profile must be a non-empty string".to_owned(),
        });
    }
    if projection == Some("") {
        return Err(TelexEncodeError {
            detail: "Telex projection must be a non-empty string".to_owned(),
        });
    }

    let mut header = VERSION_LINE.to_owned();
    if let Some(profile) = profile {
        header.push_str("\nprofile=");
        header.push_str(&encode_payload(profile));
    }
    if let Some(projection) = projection {
        header.push_str("\nprojection=");
        header.push_str(&encode_payload(projection));
    }
    if records.is_empty() {
        header.push('\n');
        return Ok(header);
    }

    let mut stanzas = Vec::with_capacity(records.len());
    for (record_index, record) in records.iter().enumerate() {
        if record.fields.is_empty() {
            return Err(TelexEncodeError {
                detail: format!("Telex record {} must not be empty", record_index + 1),
            });
        }
        let mut fields = record.fields.clone();
        let mut seen = HashSet::new();
        for (field, _) in &fields {
            if !valid_field_name(field) {
                return Err(TelexEncodeError {
                    detail: format!("Invalid Telex field name: {field}"),
                });
            }
            if !seen.insert(field.as_str()) {
                return Err(TelexEncodeError {
                    detail: format!("Duplicate Telex field: {field}"),
                });
            }
        }
        fields.sort_by(|left, right| compare_fields(&left.0, &right.0));
        let stanza = fields
            .iter()
            .map(|(field, value)| format!("{field}={}", encode_payload(value)))
            .collect::<Vec<_>>()
            .join("\n");
        stanzas.push(stanza);
    }

    Ok(format!("{header}\n\n{}\n", stanzas.join("\n\n")))
}

pub fn canonicalize_telex(input: &str) -> Result<String, TelexSyntaxError> {
    let parsed = parse_telex(input)?;
    let profile = parsed.profile_explicit.then_some(parsed.profile.as_str());
    let projection = parsed
        .projection_explicit
        .then_some(parsed.projection.as_deref())
        .flatten();
    encode_telex_with_projection(&parsed.records, profile, projection)
        .map_err(|error| TelexSyntaxError::new("TELEX_SYNTAX_ERROR", error.detail, None))
}

pub fn validate_telex(
    input: &str,
    registered_fields: &[&str],
) -> Result<ValidationResult, TelexSyntaxError> {
    let parsed = parse_telex(input)?;
    Ok(validate_telex_records_with_projection(
        &parsed.records,
        &parsed.profile,
        parsed.projection.as_deref(),
        registered_fields,
    ))
}

pub fn validate_telex_records(
    records: &[TelexRecord],
    profile: &str,
    registered_fields: &[&str],
) -> ValidationResult {
    validate_telex_records_with_projection(records, profile, None, registered_fields)
}

pub fn validate_telex_records_with_projection(
    records: &[TelexRecord],
    profile: &str,
    projection: Option<&str>,
    registered_fields: &[&str],
) -> ValidationResult {
    let registered: HashSet<&str> = registered_fields.iter().copied().collect();
    let mut diagnostics = Vec::new();
    let mut events = Vec::with_capacity(records.len());

    if profile != COMPLETE_AES_PROFILE && profile != PARTIAL_AES_PROFILE {
        diagnostics.push(Diagnostic::new(
            "AES_UNSUPPORTED_PROFILE",
            format!("Unsupported AES profile: {profile}"),
        ));
    }
    if projection.is_some_and(|value| value != AEON_DOCUMENT_PROJECTION) {
        diagnostics.push(Diagnostic::new(
            "AES_UNSUPPORTED_PROJECTION",
            format!(
                "Unsupported AES projection: {}",
                projection.unwrap_or_default()
            ),
        ));
    }

    let mut body_seen = false;
    for (index, event) in records.iter().enumerate() {
        let address_field = record_address_field(event);
        let address = address_field.and_then(|field| event.get(field.name()));
        for (field, _) in event.fields() {
            if !CORE_FIELDS.contains(&field.as_str()) && !registered.contains(field.as_str()) {
                diagnostics.push(
                    Diagnostic::new(
                        "AES_UNKNOWN_FIELD",
                        format!("Field '{field}' is not registered by profile '{profile}'"),
                    )
                    .at_record(index, address)
                    .with_field(field),
                );
            }
        }

        let has_path = event.contains("path");
        let has_header = event.contains("header");
        if !has_path && !has_header {
            diagnostics.push(
                Diagnostic::new(
                    "AES_MISSING_ADDRESS",
                    "AES records require exactly one of 'path' or 'header'",
                )
                .at_record(index, address),
            );
        } else if has_path && has_header {
            diagnostics.push(
                Diagnostic::new(
                    "AES_MULTIPLE_ADDRESSES",
                    "AES records cannot carry both 'path' and 'header'",
                )
                .at_record(index, address),
            );
        }
        if !event.contains("kind") {
            diagnostics.push(
                Diagnostic::new("AES_MISSING_FIELD", "AES records require 'kind'")
                    .at_record(index, address)
                    .with_field("kind"),
            );
        }

        let mut path_details = match address {
            Some("$") => {
                diagnostics.push(
                    Diagnostic::new("AES_INVALID_PATH", "The root is not an event path")
                        .at_record(index, address)
                        .with_field(address_field.map_or("path", AddressField::name)),
                );
                None
            }
            Some(path) => match parse_canonical_data_path(path) {
                Ok(details) => Some(details),
                Err(message) => {
                    let field = address_field.map_or("path", AddressField::name);
                    let code = if address_field == Some(AddressField::Header) {
                        "AES_INVALID_HEADER_PATH"
                    } else {
                        "AES_INVALID_PATH"
                    };
                    diagnostics.push(
                        Diagnostic::new(code, message)
                            .at_record(index, Some(path))
                            .with_field(field),
                    );
                    None
                }
            },
            None => None,
        };
        match address_field {
            Some(AddressField::Header) => {
                if projection != Some(AEON_DOCUMENT_PROJECTION) {
                    diagnostics.push(
                        Diagnostic::new(
                            "AES_HEADER_REQUIRES_PROJECTION",
                            format!(
                                "Header records require projection '{AEON_DOCUMENT_PROJECTION}'"
                            ),
                        )
                        .at_record(index, address)
                        .with_field("header"),
                    );
                }
                if body_seen {
                    diagnostics.push(
                        Diagnostic::new(
                            "AES_HEADER_ORDER",
                            "Header records must precede body events",
                        )
                        .at_record(index, address)
                        .with_field("header"),
                    );
                }
                if let (Some(path), Some(details)) = (address, &path_details)
                    && !is_aeon_header_path(path, details)
                {
                    diagnostics.push(
                        Diagnostic::new(
                            "AES_INVALID_HEADER_PATH",
                            "Header paths must begin with a quoted 'aeon:' member",
                        )
                        .at_record(index, address)
                        .with_field("header"),
                    );
                    path_details = None;
                }
            }
            Some(AddressField::Path) => body_seen = true,
            None => {}
        }

        let kind = event.get("kind");
        let known_kind = kind.is_some_and(|kind| VALUE_KINDS.contains(&kind));
        if let Some(kind) = kind.filter(|kind| !VALUE_KINDS.contains(kind)) {
            diagnostics.push(
                Diagnostic::new(
                    "AES_UNKNOWN_KIND",
                    format!("Unknown AES value kind: {kind}"),
                )
                .at_record(index, address)
                .with_field("kind"),
            );
        }
        if known_kind {
            validate_event_value(event, index, &mut diagnostics);
        }
        validate_optional_fields(event, index, &mut diagnostics);
        events.push(EventCandidate {
            event,
            index,
            address_field,
            address,
            path_details,
        });
    }

    let body_events = events
        .iter()
        .filter(|candidate| candidate.address_field == Some(AddressField::Path))
        .collect::<Vec<_>>();
    let header_events = events
        .iter()
        .filter(|candidate| candidate.address_field == Some(AddressField::Header))
        .collect::<Vec<_>>();
    if profile == COMPLETE_AES_PROFILE {
        validate_complete_stream(&body_events, &mut diagnostics);
    }
    if projection == Some(AEON_DOCUMENT_PROJECTION) {
        validate_complete_stream(&header_events, &mut diagnostics);
    }
    if profile == COMPLETE_AES_PROFILE {
        let identity_events = body_events
            .iter()
            .chain(
                (projection == Some(AEON_DOCUMENT_PROJECTION))
                    .then_some(header_events.iter())
                    .into_iter()
                    .flatten(),
            )
            .copied()
            .collect::<Vec<_>>();
        validate_identity_uniqueness(&identity_events, &mut diagnostics);
    } else if projection == Some(AEON_DOCUMENT_PROJECTION) {
        validate_identity_uniqueness(&header_events, &mut diagnostics);
    }

    ValidationResult {
        valid: diagnostics.is_empty(),
        profile: profile.to_owned(),
        diagnostics,
    }
}

fn validate_event_value(event: &TelexRecord, index: usize, diagnostics: &mut Vec<Diagnostic>) {
    let Some(kind) = event.get("kind") else {
        return;
    };
    let path = record_address(event);
    let value = event.get("value");
    if ["object", "list", "tuple", "node"].contains(&kind) {
        if value.is_some() {
            diagnostics.push(
                Diagnostic::new(
                    "AES_UNEXPECTED_VALUE",
                    format!("Kind '{kind}' must not carry 'value'"),
                )
                .at_record(index, path)
                .with_field("value"),
            );
        }
        return;
    }
    let Some(value) = value else {
        diagnostics.push(
            Diagnostic::new(
                "AES_MISSING_VALUE",
                format!("Kind '{kind}' requires 'value'"),
            )
            .at_record(index, path)
            .with_field("value"),
        );
        return;
    };

    let exact_valid = match kind {
        "infinity" => ["Infinity", "-Infinity"].contains(&value),
        "nan" => ["NaN", "-NaN"].contains(&value),
        "boolean" => ["true", "false"].contains(&value),
        "toggle" => ["yes", "no", "on", "off"].contains(&value),
        _ => true,
    };
    if !exact_valid {
        diagnostics.push(
            Diagnostic::new(
                "AES_INVALID_VALUE",
                format!("Invalid '{kind}' payload: {value}"),
            )
            .at_record(index, path)
            .with_field("value"),
        );
    }
    if kind == "hex"
        && (value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()))
    {
        diagnostics.push(
            Diagnostic::new(
                "AES_INVALID_VALUE",
                "Hex payloads require one or more lowercase hexadecimal digits",
            )
            .at_record(index, path)
            .with_field("value"),
        );
    }
    if kind == "node-head" && value.is_empty() {
        diagnostics.push(
            Diagnostic::new("AES_INVALID_VALUE", "Node tags must not be empty")
                .at_record(index, path)
                .with_field("value"),
        );
    }
    if kind == "wtc"
        && let Some((_, reference)) = value.rsplit_once('&')
        && reference.eq_ignore_ascii_case("local")
        && reference != "local"
    {
        diagnostics.push(
            Diagnostic::new(
                "AES_INVALID_VALUE",
                "The reserved WTC reference must be exact lowercase 'local'",
            )
            .at_record(index, path)
            .with_field("value"),
        );
    }
    if ["clone-reference", "pointer-reference"].contains(&kind)
        && let Err(message) = parse_canonical_data_path(value)
    {
        diagnostics.push(
            Diagnostic::new("AES_INVALID_REFERENCE", message)
                .at_record(index, path)
                .with_field("value"),
        );
    }
}

fn validate_optional_fields(event: &TelexRecord, index: usize, diagnostics: &mut Vec<Diagnostic>) {
    let path = record_address(event);
    for field in ["datatype", "identity"] {
        if event.get(field) == Some("") {
            diagnostics.push(
                Diagnostic::new(
                    "AES_EMPTY_FIELD",
                    format!("Field '{field}' must not be empty when present"),
                )
                .at_record(index, path)
                .with_field(field),
            );
        }
    }
    if let Some(origin) = event.get("origin")
        && !valid_origin(origin)
    {
        diagnostics.push(
            Diagnostic::new(
                "AES_INVALID_ORIGIN",
                "Origin must be 'sha256:' followed by 64 lowercase hexadecimal digits",
            )
            .at_record(index, path)
            .with_field("origin"),
        );
    }
    if let Some(span) = event.get("span") {
        if !event.contains("origin") {
            diagnostics.push(
                Diagnostic::new(
                    "AES_SPAN_REQUIRES_ORIGIN",
                    "Field 'span' requires source identity in 'origin'",
                )
                .at_record(index, path)
                .with_field("span"),
            );
        }
        if !valid_span(span) {
            diagnostics.push(
                Diagnostic::new(
                    "AES_INVALID_SPAN",
                    "Span must be canonical 'start-byte:end-byte' with start-byte <= end-byte",
                )
                .at_record(index, path)
                .with_field("span"),
            );
        }
    }
}

struct EventCandidate<'a> {
    event: &'a TelexRecord,
    index: usize,
    address_field: Option<AddressField>,
    address: Option<&'a str>,
    path_details: Option<PathDetails>,
}

fn validate_complete_stream(events: &[&EventCandidate<'_>], diagnostics: &mut Vec<Diagnostic>) {
    let mut by_path: HashMap<&str, &EventCandidate<'_>> = HashMap::new();

    for candidate in events {
        if candidate.path_details.is_some()
            && let Some(path) = candidate.address
        {
            if let Some(first) = by_path.get(path) {
                diagnostics.push(
                    Diagnostic::new(
                        "AES_DUPLICATE_PATH",
                        format!("Duplicate event path '{path}'"),
                    )
                    .at_record(candidate.index, Some(path))
                    .with_first_record(first.index),
                );
            } else {
                by_path.insert(path, candidate);
            }
        }
    }

    for candidate in events {
        let Some(details) = &candidate.path_details else {
            continue;
        };
        let path = candidate.address;
        let kind = candidate.event.get("kind");
        if details.segments.len() == 1 {
            if details.segments[0] != Segment::Member {
                diagnostics.push(
                    Diagnostic::new(
                        "AES_MISSING_PARENT",
                        "Only a member event can be a direct child of the unrepresented '$' root",
                    )
                    .at_record(candidate.index, path)
                    .with_required_path("$"),
                );
            }
            if kind == Some("node-head") {
                diagnostics.push(invalid_node_head(candidate));
            }
            continue;
        }

        let Some(segment) = details.segments.last() else {
            continue;
        };
        let parent_path = &details.prefixes[details.prefixes.len() - 2];
        let Some(parent) = by_path.get(parent_path.as_str()) else {
            diagnostics.push(
                Diagnostic::new(
                    "AES_MISSING_PARENT",
                    format!("Missing parent event '{parent_path}'"),
                )
                .at_record(candidate.index, path)
                .with_required_path(parent_path),
            );
            continue;
        };
        let parent_kind = parent.event.get("kind");
        match segment {
            Segment::Member if parent_kind != Some("object") => diagnostics.push(
                incompatible_parent(candidate, parent_path, parent_kind, "object"),
            ),
            Segment::Index if parent_kind == Some("node") && kind != Some("node-head") => {
                diagnostics.push(incompatible_parent(
                    candidate,
                    parent_path,
                    parent_kind,
                    "node-head child",
                ));
            }
            Segment::Index
                if ![Some("list"), Some("tuple"), Some("node"), Some("node-head")]
                    .contains(&parent_kind) =>
            {
                diagnostics.push(incompatible_parent(
                    candidate,
                    parent_path,
                    parent_kind,
                    "list, tuple, node, or node-head",
                ));
            }
            Segment::Attribute | Segment::Member | Segment::Index => {}
        }
        if kind == Some("node-head") && (*segment != Segment::Index || parent_kind != Some("node"))
        {
            diagnostics.push(invalid_node_head(candidate));
        }
    }
}

fn validate_identity_uniqueness(events: &[&EventCandidate<'_>], diagnostics: &mut Vec<Diagnostic>) {
    let mut identities: HashMap<&str, usize> = HashMap::new();
    for candidate in events {
        let Some(identity) = candidate
            .event
            .get("identity")
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if let Some(first) = identities.get(identity) {
            diagnostics.push(
                Diagnostic::new(
                    "AES_DUPLICATE_IDENTITY",
                    format!("Duplicate structural identity '{identity}'"),
                )
                .at_record(candidate.index, candidate.address)
                .with_field("identity")
                .with_first_record(*first),
            );
        } else {
            identities.insert(identity, candidate.index);
        }
    }
}

fn incompatible_parent(
    candidate: &EventCandidate<'_>,
    parent_path: &str,
    actual: Option<&str>,
    expected: &str,
) -> Diagnostic {
    Diagnostic::new(
        "AES_INCOMPATIBLE_PARENT",
        format!(
            "Parent '{parent_path}' has kind '{}'; expected {expected}",
            actual.unwrap_or("")
        ),
    )
    .at_record(candidate.index, candidate.address)
    .with_required_path(parent_path)
}

fn invalid_node_head(candidate: &EventCandidate<'_>) -> Diagnostic {
    Diagnostic::new(
        "AES_INVALID_NODE_HEAD",
        "A 'node-head' must be an indexed direct child of a 'node'",
    )
    .at_record(candidate.index, candidate.address)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddressField {
    Path,
    Header,
}

impl AddressField {
    const fn name(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Header => "header",
        }
    }
}

fn record_address_field(record: &TelexRecord) -> Option<AddressField> {
    match (record.contains("path"), record.contains("header")) {
        (true, false) => Some(AddressField::Path),
        (false, true) => Some(AddressField::Header),
        _ => None,
    }
}

fn record_address(record: &TelexRecord) -> Option<&str> {
    record_address_field(record).and_then(|field| record.get(field.name()))
}

fn is_aeon_header_path(path: &str, details: &PathDetails) -> bool {
    if details.segments.first() != Some(&Segment::Member) {
        return false;
    }
    let Some(first) = details
        .prefixes
        .first()
        .filter(|prefix| prefix.starts_with("$.["))
    else {
        return false;
    };
    decode_json_string(first, 3)
        .is_ok_and(|(member, _)| member.starts_with("aeon:") && member.len() > 5)
        && path.starts_with(first)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Segment {
    Member,
    Index,
    Attribute,
}

#[derive(Debug, Clone)]
struct PathDetails {
    prefixes: Vec<String>,
    segments: Vec<Segment>,
}

fn parse_canonical_data_path(path: &str) -> Result<PathDetails, String> {
    if !path.starts_with('$') {
        return Err(format!("Expected an absolute canonical path: {path}"));
    }
    if path == "$" {
        return Ok(PathDetails {
            prefixes: Vec::new(),
            segments: Vec::new(),
        });
    }

    let bytes = path.as_bytes();
    let mut cursor = 1_usize;
    let mut current = "$".to_owned();
    let mut prefixes = Vec::new();
    let mut segments = Vec::new();
    while cursor < bytes.len() {
        let start = cursor;
        let segment = if path[cursor..].starts_with(".@.") {
            cursor += 3;
            cursor = read_member_end(path, cursor)?;
            Segment::Attribute
        } else if bytes[cursor] == b'.' {
            cursor += 1;
            cursor = read_member_end(path, cursor)?;
            Segment::Member
        } else if bytes[cursor] == b'[' {
            cursor = read_index_end(path, cursor)?;
            Segment::Index
        } else {
            return Err(format!("Invalid canonical path segment in: {path}"));
        };
        current.push_str(&path[start..cursor]);
        prefixes.push(current.clone());
        segments.push(segment);
    }
    Ok(PathDetails { prefixes, segments })
}

fn read_member_end(path: &str, cursor: usize) -> Result<usize, String> {
    let bytes = path.as_bytes();
    if bytes.get(cursor) == Some(&b'[') {
        if bytes.get(cursor + 1) != Some(&b'"') {
            return Err(format!(
                "Expected a quoted canonical member in path: {path}"
            ));
        }
        let (decoded, quote_end) = decode_json_string(path, cursor + 1)?;
        if bytes.get(quote_end + 1) != Some(&b']') {
            return Err(format!(
                "Unterminated quoted canonical member in path: {path}"
            ));
        }
        let encoded = &path[cursor + 1..=quote_end];
        if decoded.is_empty()
            || valid_bare_member(&decoded)
            || canonical_json_string(&decoded) != encoded
        {
            return Err(format!("Non-canonical quoted member in path: {path}"));
        }
        return Ok(quote_end + 2);
    }

    let Some(first) = bytes.get(cursor).copied() else {
        return Err(format!("Invalid canonical member in path: {path}"));
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return Err(format!("Invalid canonical member in path: {path}"));
    }
    let mut end = cursor + 1;
    while bytes
        .get(end)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        end += 1;
    }
    Ok(end)
}

fn valid_bare_member(member: &str) -> bool {
    let mut bytes = member.bytes();
    bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn read_index_end(path: &str, cursor: usize) -> Result<usize, String> {
    let bytes = path.as_bytes();
    let mut end = cursor + 1;
    let digit_start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    let digits = &path[digit_start..end];
    if digits.is_empty()
        || (digits.len() > 1 && digits.starts_with('0'))
        || bytes.get(end) != Some(&b']')
    {
        return Err(format!("Invalid canonical index in path: {path}"));
    }
    Ok(end + 1)
}

fn decode_json_string(input: &str, quote_start: usize) -> Result<(String, usize), String> {
    let bytes = input.as_bytes();
    let mut cursor = quote_start + 1;
    let mut decoded = String::new();
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' => return Ok((decoded, cursor)),
            b'\\' => {
                cursor += 1;
                let Some(escape) = bytes.get(cursor).copied() else {
                    break;
                };
                match escape {
                    b'"' => decoded.push('"'),
                    b'\\' => decoded.push('\\'),
                    b'/' => decoded.push('/'),
                    b'b' => decoded.push('\u{0008}'),
                    b'f' => decoded.push('\u{000c}'),
                    b'n' => decoded.push('\n'),
                    b'r' => decoded.push('\r'),
                    b't' => decoded.push('\t'),
                    b'u' => {
                        let (scalar, next) = decode_json_unicode_escape(input, cursor + 1)?;
                        decoded.push(scalar);
                        cursor = next - 1;
                    }
                    _ => return Err(format!("Invalid quoted canonical member in path: {input}")),
                }
                cursor += 1;
            }
            byte if byte < 0x20 => {
                return Err(format!("Invalid quoted canonical member in path: {input}"));
            }
            _ => {
                let Some(character) = input[cursor..].chars().next() else {
                    break;
                };
                decoded.push(character);
                cursor += character.len_utf8();
            }
        }
    }
    Err(format!(
        "Unterminated quoted canonical member in path: {input}"
    ))
}

fn decode_json_unicode_escape(input: &str, digits_start: usize) -> Result<(char, usize), String> {
    let first_end = digits_start + 4;
    let Some(first_digits) = input.get(digits_start..first_end) else {
        return Err(format!("Invalid quoted canonical member in path: {input}"));
    };
    let first = u16::from_str_radix(first_digits, 16)
        .map_err(|_| format!("Invalid quoted canonical member in path: {input}"))?;
    if (0xd800..=0xdbff).contains(&first) {
        let second_prefix_end = first_end + 2;
        if input.get(first_end..second_prefix_end) != Some("\\u") {
            return Err(format!("Invalid quoted canonical member in path: {input}"));
        }
        let second_end = second_prefix_end + 4;
        let Some(second_digits) = input.get(second_prefix_end..second_end) else {
            return Err(format!("Invalid quoted canonical member in path: {input}"));
        };
        let second = u16::from_str_radix(second_digits, 16)
            .map_err(|_| format!("Invalid quoted canonical member in path: {input}"))?;
        if !(0xdc00..=0xdfff).contains(&second) {
            return Err(format!("Invalid quoted canonical member in path: {input}"));
        }
        let scalar = 0x1_0000 + ((u32::from(first) - 0xd800) << 10) + (u32::from(second) - 0xdc00);
        let character = char::from_u32(scalar)
            .ok_or_else(|| format!("Invalid quoted canonical member in path: {input}"))?;
        return Ok((character, second_end));
    }
    if (0xdc00..=0xdfff).contains(&first) {
        return Err(format!("Invalid quoted canonical member in path: {input}"));
    }
    let character = char::from_u32(u32::from(first))
        .ok_or_else(|| format!("Invalid quoted canonical member in path: {input}"))?;
    Ok((character, first_end))
}

fn canonical_json_string(value: &str) -> String {
    let mut encoded = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => encoded.push_str("\\\""),
            '\\' => encoded.push_str("\\\\"),
            '\u{0008}' => encoded.push_str("\\b"),
            '\u{000c}' => encoded.push_str("\\f"),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            character if character <= '\u{001f}' => {
                encoded.push_str(&format!("\\u{:04x}", u32::from(character)));
            }
            character => encoded.push(character),
        }
    }
    encoded.push('"');
    encoded
}

fn valid_span(span: &str) -> bool {
    let Some((start, end)) = span.split_once(':') else {
        return false;
    };
    if end.contains(':') || !canonical_unsigned(start) || !canonical_unsigned(end) {
        return false;
    }
    start.len() < end.len() || (start.len() == end.len() && start <= end)
}

fn valid_origin(origin: &str) -> bool {
    origin.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn canonical_unsigned(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'))
}

fn valid_field_name(field: &str) -> bool {
    field.split('.').all(|segment| {
        let mut bytes = segment.bytes();
        bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
            && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    })
}

fn compare_fields(left: &str, right: &str) -> Ordering {
    match (core_rank(left), core_rank(right)) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.cmp(right),
    }
}

fn core_rank(field: &str) -> Option<usize> {
    CORE_FIELDS.iter().position(|candidate| *candidate == field)
}

fn has_canonical_field_order(record: &TelexRecord) -> bool {
    record
        .fields
        .windows(2)
        .all(|pair| compare_fields(&pair[0].0, &pair[1].0) != Ordering::Greater)
}

struct DecodedPayload {
    value: String,
    canonical: bool,
}

fn decode_payload(payload: &str, line: usize) -> Result<DecodedPayload, TelexSyntaxError> {
    let mut value = String::new();
    let mut canonical = true;
    let mut characters = payload.char_indices().peekable();
    while let Some((_, character)) = characters.next() {
        if character != '\\' {
            let scalar = u32::from(character);
            if scalar <= 0x1f || scalar == 0x7f {
                return Err(TelexSyntaxError::new(
                    "TELEX_UNESCAPED_CONTROL",
                    "Unescaped control character in payload",
                    Some(line),
                ));
            }
            value.push(character);
            continue;
        }

        let Some((_, escape)) = characters.next() else {
            return Err(TelexSyntaxError::new(
                "TELEX_INCOMPLETE_ESCAPE",
                "Incomplete escape",
                Some(line),
            ));
        };
        match escape {
            '\\' => value.push('\\'),
            'n' => value.push('\n'),
            'r' => value.push('\r'),
            't' => value.push('\t'),
            '0' => value.push('\0'),
            'u' => {
                if characters.next().map(|(_, character)| character) != Some('{') {
                    return Err(TelexSyntaxError::new(
                        "TELEX_UNKNOWN_ESCAPE",
                        "Unknown escape: \\u",
                        Some(line),
                    ));
                }
                let mut digits = String::new();
                let mut closed = false;
                for (_, character) in characters.by_ref() {
                    if character == '}' {
                        closed = true;
                        break;
                    }
                    digits.push(character);
                }
                if !closed {
                    return Err(TelexSyntaxError::new(
                        "TELEX_UNTERMINATED_UNICODE_ESCAPE",
                        "Unterminated Unicode escape",
                        Some(line),
                    ));
                }
                if digits.is_empty()
                    || digits.len() > 6
                    || !digits.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return Err(TelexSyntaxError::new(
                        "TELEX_INVALID_UNICODE_ESCAPE",
                        "Invalid Unicode escape",
                        Some(line),
                    ));
                }
                let scalar = u32::from_str_radix(&digits, 16).map_err(|_| {
                    TelexSyntaxError::new(
                        "TELEX_INVALID_UNICODE_ESCAPE",
                        "Invalid Unicode escape",
                        Some(line),
                    )
                })?;
                let Some(character) = char::from_u32(scalar) else {
                    return Err(TelexSyntaxError::new(
                        "TELEX_INVALID_UNICODE_SCALAR",
                        "Unicode escape is not a scalar value",
                        Some(line),
                    ));
                };
                canonical &= digits == format!("{scalar:X}");
                if matches!(scalar, 0 | 9 | 10 | 13) {
                    canonical = false;
                }
                value.push(character);
            }
            _ => {
                return Err(TelexSyntaxError::new(
                    "TELEX_UNKNOWN_ESCAPE",
                    format!("Unknown escape: \\{escape}"),
                    Some(line),
                ));
            }
        }
    }
    Ok(DecodedPayload { value, canonical })
}

fn encode_payload(payload: &str) -> String {
    let mut encoded = String::new();
    for character in payload.chars() {
        match character {
            '\\' => encoded.push_str("\\\\"),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            '\0' => encoded.push_str("\\0"),
            character if character <= '\u{001f}' || character == '\u{007f}' => {
                encoded.push_str(&format!("\\u{{{:X}}}", u32::from(character)));
            }
            character => encoded.push(character),
        }
    }
    encoded
}
