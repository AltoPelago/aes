//! Selected Film v1 draft reference codec.
//!
//! This module implements the selected table-free Film v1 layout. The format
//! and its conformance suite remain mutable drafts rather than a released
//! conformance target. Comparator encodings must use a different preamble.

use std::error::Error;
use std::fmt;

use crate::{
    COMPLETE_AES_PROFILE, ClarifierKind, DatatypeClarifier, DatatypeDescriptor, Diagnostic,
    GenericArgument, ParsedTelex, TelexLimits, TelexRecord, encode_telex_with_projection,
    limit_diagnostic, parse_telex, validate_telex_records_with_projection_and_limits,
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

/// Explicit name for Film data whose field storage is owned. Ownership alone
/// does not imply AES validation; use [`decode_film`] or
/// [`FilmStreamView::to_validated_owned`] when semantic acceptance is required.
pub type OwnedFilmStream = FilmStream;

/// A decoded address that borrows its UTF-8 bytes from the Film input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilmAddressView<'a> {
    Path(&'a str),
    Header(&'a str),
}

impl FilmAddressView<'_> {
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Path(_) => "path",
            Self::Header(_) => "header",
        }
    }

    #[must_use]
    pub fn value(&self) -> &str {
        match self {
            Self::Path(value) | Self::Header(value) => value,
        }
    }
}

/// A datatype generic that retains borrowed Film string storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilmGenericView<'a> {
    Datatype(FilmDatatypeView<'a>),
    NumberLiteral(&'a str),
}

/// A datatype clarifier that retains borrowed Film string storage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilmClarifierView<'a> {
    pub kind: ClarifierKind,
    pub value: &'a str,
}

/// A recursive datatype descriptor whose text fields borrow from Film input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilmDatatypeView<'a> {
    pub datatype: &'a str,
    pub generics: Vec<FilmGenericView<'a>>,
    pub clarifiers: Vec<FilmClarifierView<'a>>,
}

impl FilmDatatypeView<'_> {
    #[must_use]
    pub fn to_owned(&self) -> DatatypeDescriptor {
        DatatypeDescriptor {
            datatype: self.datatype.to_owned(),
            generics: self
                .generics
                .iter()
                .map(|generic| match generic {
                    FilmGenericView::Datatype(datatype) => {
                        GenericArgument::Datatype(datatype.to_owned())
                    }
                    FilmGenericView::NumberLiteral(value) => {
                        GenericArgument::NumberLiteral((*value).to_owned())
                    }
                })
                .collect(),
            clarifiers: self
                .clarifiers
                .iter()
                .map(|clarifier| DatatypeClarifier {
                    kind: clarifier.kind.clone(),
                    value: clarifier.value.to_owned(),
                })
                .collect(),
        }
    }
}

/// One named extension whose name and value borrow from Film input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilmExtensionView<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

/// A physically decoded Film record. AES event and stream validation remain
/// provisional until the view is materialized and validated as a complete
/// stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilmRecordView<'a> {
    pub address: FilmAddressView<'a>,
    pub kind: &'static str,
    pub datatype: Option<FilmDatatypeView<'a>>,
    pub identity: Option<&'a str>,
    pub value: Option<&'a str>,
    pub origin: Option<&'a [u8; 32]>,
    pub span: Option<(u64, u64)>,
    pub extensions: Vec<FilmExtensionView<'a>>,
}

impl FilmRecordView<'_> {
    #[must_use]
    pub fn to_owned(&self) -> TelexRecord {
        let mut fields = vec![
            (
                self.address.name().to_owned(),
                self.address.value().to_owned(),
            ),
            ("kind".to_owned(), self.kind.to_owned()),
        ];
        if let Some(datatype) = &self.datatype {
            fields.push(("datatype".to_owned(), datatype.datatype.to_owned()));
        }
        if let Some(identity) = self.identity {
            fields.push(("identity".to_owned(), identity.to_owned()));
        }
        if let Some(value) = self.value {
            fields.push(("value".to_owned(), value.to_owned()));
        }
        if let Some(origin) = self.origin {
            fields.push(("origin".to_owned(), encode_origin(origin)));
        }
        if let Some((start, end)) = self.span {
            fields.push(("span".to_owned(), format!("{start}:{end}")));
        }
        fields.extend(
            self.extensions
                .iter()
                .map(|extension| (extension.name.to_owned(), extension.value.to_owned())),
        );
        match &self.datatype {
            Some(datatype) => TelexRecord::with_datatype(fields, datatype.to_owned()),
            None => TelexRecord::new(fields),
        }
    }
}

/// A Film stream view whose encoded string fields and origin digests borrow
/// from the caller's input buffer. An omitted profile uses the static AES
/// default. The view has passed Film framing, syntax, and canonicality checks,
/// but remains provisional with respect to AES event and stream semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilmStreamView<'a> {
    pub profile: &'a str,
    pub profile_explicit: bool,
    pub projection: Option<&'a str>,
    pub projection_explicit: bool,
    pub records: Vec<FilmRecordView<'a>>,
    input_bytes: usize,
}

impl FilmStreamView<'_> {
    #[must_use]
    pub fn input_bytes(&self) -> usize {
        self.input_bytes
    }

    #[must_use]
    pub fn to_owned_unvalidated(&self) -> OwnedFilmStream {
        FilmStream {
            profile: self.profile.to_owned(),
            profile_explicit: self.profile_explicit,
            projection: self.projection.map(str::to_owned),
            projection_explicit: self.projection_explicit,
            records: self.records.iter().map(FilmRecordView::to_owned).collect(),
        }
    }

    /// Materializes borrowed fields and performs complete AES validation under
    /// the selected profile and projection.
    pub fn to_validated_owned(
        &self,
        registered_fields: &[&str],
        aes_limits: &TelexLimits,
    ) -> Result<OwnedFilmStream, FilmError> {
        let stream = self.to_owned_unvalidated();
        validate_stream(&stream, registered_fields, aes_limits, self.input_bytes)?;
        Ok(stream)
    }
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
pub fn film_v1_is_draft() -> bool {
    true
}

#[must_use]
pub fn candidate_a_is_research_only() -> bool {
    film_v1_is_draft()
}

pub fn encode_film(stream: &FilmStream, registered_fields: &[&str]) -> Result<Vec<u8>, FilmError> {
    encode_film_with_limits(
        stream,
        registered_fields,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
}

pub fn encode_film_with_limits(
    stream: &FilmStream,
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<Vec<u8>, FilmError> {
    validate_context(stream)?;
    validate_stream(stream, registered_fields, aes_limits, 0)?;

    let mut context_size = FILM_V1_PREAMBLE.len().saturating_add(1);
    if stream.profile_explicit {
        context_size = checked_encoded_size(
            context_size,
            encoded_string_size(&stream.profile, film_limits, "profile", None)?,
            None,
            "profile",
        )?;
    }
    if stream.projection_explicit {
        context_size = checked_encoded_size(
            context_size,
            encoded_string_size(
                stream.projection.as_deref().unwrap_or_default(),
                film_limits,
                "projection",
                None,
            )?,
            None,
            "projection",
        )?;
    }
    enforce_limit(
        "max_input_bytes",
        context_size,
        film_limits.max_input_bytes,
        0,
        None,
        "stream",
    )?;
    let sizing_plan_bytes = stream
        .records
        .len()
        .checked_mul(std::mem::size_of::<usize>())
        .ok_or_else(|| {
            film_error(
                "FILM_ENCODE_ERROR",
                0,
                None,
                "sizing-plan",
                "Film record sizing plan does not fit the platform size domain",
            )
        })?;
    enforce_limit(
        "max_buffered_bytes",
        sizing_plan_bytes,
        film_limits.max_buffered_bytes,
        0,
        None,
        "sizing-plan",
    )?;
    let mut payload_sizes = Vec::new();
    payload_sizes
        .try_reserve_exact(stream.records.len())
        .map_err(|_| {
            film_error(
                "FILM_ENCODE_ERROR",
                0,
                None,
                "sizing-plan",
                "Film record sizing plan cannot be allocated in the platform size domain",
            )
        })?;
    let mut encoded_size = context_size;
    for (record_index, record) in stream.records.iter().enumerate() {
        let payload_size = encoded_record_size(record, record_index, film_limits, aes_limits)?;
        enforce_limit(
            "max_record_bytes",
            payload_size,
            film_limits.max_record_bytes,
            encoded_size,
            Some(record_index),
            "record",
        )?;
        enforce_limit(
            "max_buffered_bytes",
            payload_size,
            film_limits.max_buffered_bytes,
            encoded_size,
            Some(record_index),
            "record",
        )?;
        let payload_length = usize_to_u64(payload_size, encoded_size, record_index)?;
        let framed_size = checked_encoded_size(
            uleb_width(payload_length),
            payload_size,
            Some(record_index),
            "record",
        )?;
        encoded_size =
            checked_encoded_size(encoded_size, framed_size, Some(record_index), "stream")?;
        enforce_limit(
            "max_input_bytes",
            encoded_size,
            film_limits.max_input_bytes,
            encoded_size.saturating_sub(framed_size),
            Some(record_index),
            "stream",
        )?;
        payload_sizes.push(payload_size);
    }

    let mut output = encode_buffer_with_capacity(encoded_size, None, "stream")?;
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
    debug_assert_eq!(output.len(), context_size);

    for (record_index, (record, payload_size)) in
        stream.records.iter().zip(payload_sizes).enumerate()
    {
        let payload_length = usize_to_u64(payload_size, output.len(), record_index)?;
        push_uleb(&mut output, payload_length);
        let payload_start = output.len();
        encode_record_into(&mut output, record, record_index, film_limits, aes_limits)?;
        debug_assert_eq!(output.len() - payload_start, payload_size);
    }
    debug_assert_eq!(output.len(), encoded_size);

    Ok(output)
}

pub fn decode_film(input: &[u8], registered_fields: &[&str]) -> Result<FilmStream, FilmError> {
    decode_film_with_limits(
        input,
        registered_fields,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
}

pub fn decode_film_with_limits(
    input: &[u8],
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<FilmStream, FilmError> {
    decode_film_view_with_limits(input, film_limits, aes_limits)?
        .to_validated_owned(registered_fields, aes_limits)
}

/// Decodes Film framing, fields, and canonical physical form while borrowing
/// string and origin storage from `input`. The returned stream remains
/// provisional until AES validation succeeds or it is passed through
/// [`decode_film`].
pub fn decode_film_view(input: &[u8]) -> Result<FilmStreamView<'_>, FilmError> {
    decode_film_view_with_limits(input, &FilmLimits::default(), &TelexLimits::default())
}

/// Limit-aware borrowed Film decoding. Film-local limits apply to physical
/// bytes and shared AES limits bound structural parsing, but this function does
/// not perform profile, projection, path, extension-registration, or other AES
/// semantic validation.
pub fn decode_film_view_with_limits<'a>(
    input: &'a [u8],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<FilmStreamView<'a>, FilmError> {
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
            "FILM_INVALID_CONTEXT",
            reader.absolute_position().saturating_sub(1),
            None,
            "stream-context",
            "Reserved stream-context bits must be zero",
        ));
    }
    let profile_explicit = context & CONTEXT_PROFILE != 0;
    let projection_explicit = context & CONTEXT_PROJECTION != 0;
    let profile = if profile_explicit {
        reader.read_nonempty_context_str(film_limits, "profile")?
    } else {
        COMPLETE_AES_PROFILE
    };
    let projection = if projection_explicit {
        Some(reader.read_nonempty_context_str(film_limits, "projection")?)
    } else {
        None
    };

    let mut records = Vec::new();
    while !reader.is_empty() {
        let record_index = records.len();
        if record_index >= aes_limits.max_events {
            return Err(aes_limit_error(
                "max_events",
                record_index.saturating_add(1),
                aes_limits.max_events,
                reader.absolute_position(),
                Some(record_index),
                "record",
            ));
        }
        reader.record = Some(record_index);
        let payload_length = reader.read_limited_length(
            "max_record_bytes",
            film_limits.max_record_bytes,
            "record-length",
            "record",
        )?;
        if payload_length == 0 {
            return Err(film_error(
                "FILM_NONCANONICAL",
                reader.absolute_position(),
                Some(record_index),
                "record-length",
                "Film record payload length must be positive",
            ));
        }
        let payload_offset = reader.absolute_position();
        let payload = reader.read_exact(payload_length, "record")?;
        records.push(decode_record_view(
            payload,
            payload_offset,
            record_index,
            film_limits,
            aes_limits,
        )?);
    }

    Ok(FilmStreamView {
        profile,
        profile_explicit,
        projection,
        projection_explicit,
        records,
        input_bytes: input.len(),
    })
}

pub fn telex_to_film(telex: &str, registered_fields: &[&str]) -> Result<Vec<u8>, FilmError> {
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
    encode_film(&FilmStream::from(&parsed), registered_fields)
}

pub fn film_to_telex(film: &[u8], registered_fields: &[&str]) -> Result<String, FilmError> {
    let stream = decode_film(film, registered_fields)?;
    let profile = stream.profile_explicit.then_some(stream.profile.as_str());
    let projection = stream
        .projection_explicit
        .then_some(stream.projection.as_deref())
        .flatten();
    encode_telex_with_projection(&stream.records, profile, projection)
        .map_err(|error| film_error(error.code, 0, None, "telex-target", error.detail))
}

// Temporary compatibility names for the pre-selection prototype API. The
// crate is unpublished, but retaining these wrappers keeps local experiments
// reproducible while callers move to the specification-shaped names above.
pub fn encode_film_candidate_a(
    stream: &FilmStream,
    registered_fields: &[&str],
) -> Result<Vec<u8>, FilmError> {
    encode_film(stream, registered_fields)
}

pub fn encode_film_candidate_a_with_limits(
    stream: &FilmStream,
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<Vec<u8>, FilmError> {
    encode_film_with_limits(stream, registered_fields, film_limits, aes_limits)
}

pub fn decode_film_candidate_a(
    input: &[u8],
    registered_fields: &[&str],
) -> Result<FilmStream, FilmError> {
    decode_film(input, registered_fields)
}

pub fn decode_film_candidate_a_with_limits(
    input: &[u8],
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<FilmStream, FilmError> {
    decode_film_with_limits(input, registered_fields, film_limits, aes_limits)
}

pub fn telex_to_film_candidate_a(
    telex: &str,
    registered_fields: &[&str],
) -> Result<Vec<u8>, FilmError> {
    telex_to_film(telex, registered_fields)
}

pub fn film_candidate_a_to_telex(
    film: &[u8],
    registered_fields: &[&str],
) -> Result<String, FilmError> {
    film_to_telex(film, registered_fields)
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

fn checked_encoded_size(
    current: usize,
    added: usize,
    record: Option<usize>,
    component: &'static str,
) -> Result<usize, FilmError> {
    current.checked_add(added).ok_or_else(|| {
        film_error(
            "FILM_ENCODE_ERROR",
            current,
            record,
            component,
            "Encoded Film size does not fit the platform size domain",
        )
    })
}

fn encode_buffer_with_capacity(
    capacity: usize,
    record: Option<usize>,
    component: &'static str,
) -> Result<Vec<u8>, FilmError> {
    let mut output = Vec::new();
    output.try_reserve_exact(capacity).map_err(|_| {
        film_error(
            "FILM_ENCODE_ERROR",
            0,
            record,
            component,
            "Encoded Film buffer cannot be allocated in the platform size domain",
        )
    })?;
    Ok(output)
}

fn encoded_string_size(
    value: &str,
    limits: &FilmLimits,
    component: &'static str,
    record: Option<usize>,
) -> Result<usize, FilmError> {
    enforce_limit(
        "max_field_bytes",
        value.len(),
        limits.max_field_bytes,
        0,
        record,
        component,
    )?;
    let length = u64::try_from(value.len()).map_err(|_| {
        film_error(
            "FILM_ENCODE_ERROR",
            0,
            record,
            component,
            "String length does not fit the Film u64 domain",
        )
    })?;
    checked_encoded_size(uleb_width(length), value.len(), record, component)
}

fn encoded_descriptor_body_size(
    descriptor: &DatatypeDescriptor,
    depth: usize,
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
    record_index: usize,
) -> Result<usize, FilmError> {
    if depth > aes_limits.max_generic_depth {
        return Err(film_error(
            "FILM_ENCODE_ERROR",
            0,
            Some(record_index),
            "datatype",
            "Datatype generic depth exceeds the active AES limit",
        ));
    }

    let mut size = encoded_string_size(
        &descriptor.datatype,
        film_limits,
        "datatype-name",
        Some(record_index),
    )?;
    let generic_count = usize_to_u64(descriptor.generics.len(), 0, record_index)?;
    size = checked_encoded_size(
        size,
        uleb_width(generic_count),
        Some(record_index),
        "datatype",
    )?;
    for generic in &descriptor.generics {
        size = checked_encoded_size(size, 1, Some(record_index), "datatype")?;
        let generic_size = match generic {
            GenericArgument::Datatype(nested) => encoded_descriptor_size(
                nested,
                depth.saturating_add(1),
                film_limits,
                aes_limits,
                record_index,
            )?,
            GenericArgument::NumberLiteral(value) => {
                encoded_string_size(value, film_limits, "generic-number", Some(record_index))?
            }
        };
        size = checked_encoded_size(size, generic_size, Some(record_index), "datatype")?;
    }

    let clarifier_count = usize_to_u64(descriptor.clarifiers.len(), 0, record_index)?;
    size = checked_encoded_size(
        size,
        uleb_width(clarifier_count),
        Some(record_index),
        "datatype",
    )?;
    for clarifier in &descriptor.clarifiers {
        size = checked_encoded_size(size, 1, Some(record_index), "datatype")?;
        size = checked_encoded_size(
            size,
            encoded_string_size(
                &clarifier.value,
                film_limits,
                "clarifier-value",
                Some(record_index),
            )?,
            Some(record_index),
            "datatype",
        )?;
    }
    enforce_limit(
        "max_field_bytes",
        size,
        film_limits.max_field_bytes,
        0,
        Some(record_index),
        "datatype",
    )?;
    Ok(size)
}

fn encoded_descriptor_size(
    descriptor: &DatatypeDescriptor,
    depth: usize,
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
    record_index: usize,
) -> Result<usize, FilmError> {
    let body_size =
        encoded_descriptor_body_size(descriptor, depth, film_limits, aes_limits, record_index)?;
    let body_length = usize_to_u64(body_size, 0, record_index)?;
    checked_encoded_size(
        uleb_width(body_length),
        body_size,
        Some(record_index),
        "datatype",
    )
}

fn encoded_record_size(
    record: &TelexRecord,
    record_index: usize,
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<usize, FilmError> {
    reject_duplicate_fields(record, record_index)?;
    let address = match (record.get("path"), record.get("header")) {
        (Some(path), None) => path,
        (None, Some(header)) => header,
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
    if kind_code(kind).is_none() {
        return Err(film_error(
            "FILM_ENCODE_ERROR",
            0,
            Some(record_index),
            "kind",
            format!("Unassigned Film v1 kind: {kind}"),
        ));
    }

    let mut size = checked_encoded_size(
        2,
        encoded_string_size(address, film_limits, "address", Some(record_index))?,
        Some(record_index),
        "record",
    )?;
    if let Some(descriptor) = record.datatype() {
        size = checked_encoded_size(
            size,
            encoded_descriptor_size(descriptor, 0, film_limits, aes_limits, record_index)?,
            Some(record_index),
            "record",
        )?;
    }
    if let Some(identity) = record.get("identity") {
        size = checked_encoded_size(
            size,
            encoded_string_size(identity, film_limits, "identity", Some(record_index))?,
            Some(record_index),
            "record",
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
        size = checked_encoded_size(
            size,
            encoded_string_size(value, film_limits, "value", Some(record_index))?,
            Some(record_index),
            "record",
        )?;
    }
    if let Some(origin) = record.get("origin") {
        decode_origin(origin, record_index)?;
        size = checked_encoded_size(size, 32, Some(record_index), "record")?;
    }
    if let Some(span) = record.get("span") {
        let (start, end) = decode_span(span, record_index)?;
        size = checked_encoded_size(size, uleb_width(start), Some(record_index), "record")?;
        size = checked_encoded_size(size, uleb_width(end), Some(record_index), "record")?;
    }

    for (name, value) in record
        .fields()
        .iter()
        .filter(|(name, _)| !is_core_field(name))
    {
        if !valid_extension_name(name) {
            return Err(film_error(
                "FILM_ENCODE_ERROR",
                0,
                Some(record_index),
                "extension-name",
                format!("Invalid Film extension name: {name}"),
            ));
        }
        size = checked_encoded_size(
            size,
            encoded_string_size(name, film_limits, "extension-name", Some(record_index))?,
            Some(record_index),
            "record",
        )?;
        size = checked_encoded_size(
            size,
            encoded_string_size(value, film_limits, "extension-value", Some(record_index))?,
            Some(record_index),
            "record",
        )?;
    }
    Ok(size)
}

fn encode_record_into(
    output: &mut Vec<u8>,
    record: &TelexRecord,
    record_index: usize,
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<(), FilmError> {
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

    output.extend_from_slice(&[control, kind_code]);
    push_string(output, address, film_limits, "address", Some(record_index))?;
    if let Some(descriptor) = datatype {
        push_descriptor(output, descriptor, 0, film_limits, aes_limits, record_index)?;
    }
    if let Some(identity) = identity {
        push_string(
            output,
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
        push_string(output, value, film_limits, "value", Some(record_index))?;
    }
    if let Some(origin) = origin {
        output.extend_from_slice(&decode_origin(origin, record_index)?);
    }
    if let Some(span) = span {
        let (start, end) = decode_span(span, record_index)?;
        push_uleb(output, start);
        push_uleb(output, end);
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
            output,
            name,
            film_limits,
            "extension-name",
            Some(record_index),
        )?;
        push_string(
            output,
            value,
            film_limits,
            "extension-value",
            Some(record_index),
        )?;
    }
    Ok(())
}

fn decode_record_view<'a>(
    payload: &'a [u8],
    payload_offset: usize,
    record_index: usize,
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<FilmRecordView<'a>, FilmError> {
    let mut reader = Reader::new(payload, payload_offset, Some(record_index));
    let control = reader.read_byte("record-control")?;
    if control & !0x1f != 0 {
        return Err(film_error(
            "FILM_INVALID_RECORD",
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
    let address_value = reader.read_str(film_limits, "address")?;
    let address = if control & RECORD_HEADER != 0 {
        FilmAddressView::Header(address_value)
    } else {
        FilmAddressView::Path(address_value)
    };
    let mut datatype_components = 0_usize;
    let datatype = if control & RECORD_DATATYPE != 0 {
        Some(reader.read_descriptor_view(film_limits, aes_limits, 0, &mut datatype_components)?)
    } else {
        None
    };
    let identity = if control & RECORD_IDENTITY != 0 {
        Some(reader.read_str(film_limits, "identity")?)
    } else {
        None
    };
    let value = if kind_has_value(kind) {
        Some(reader.read_str(film_limits, "value")?)
    } else {
        None
    };
    let origin = if control & RECORD_ORIGIN != 0 {
        let bytes = reader.read_exact(32, "origin")?;
        Some(<&[u8; 32]>::try_from(bytes).map_err(|_| {
            film_error(
                "FILM_INVALID_RECORD",
                reader.absolute_position(),
                Some(record_index),
                "origin",
                "Film origin must contain exactly 32 bytes",
            )
        })?)
    } else {
        None
    };
    let span = if control & RECORD_SPAN != 0 {
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
        Some((start, end))
    } else {
        None
    };

    let mut previous_extension: Option<&str> = None;
    let mut extensions = Vec::new();
    while !reader.is_empty() {
        let name_offset = reader.absolute_position();
        let name = reader.read_str(film_limits, "extension-name")?;
        if !valid_extension_name(name) {
            return Err(film_error(
                "FILM_INVALID_EXTENSION",
                name_offset,
                Some(record_index),
                "extension-name",
                format!("Invalid Film extension name: {name}"),
            ));
        }
        if previous_extension.is_some_and(|previous| previous >= name) {
            return Err(film_error(
                "FILM_NONCANONICAL",
                name_offset,
                Some(record_index),
                "extension-name",
                "Extensions must be strictly ordered without duplicates",
            ));
        }
        let value = reader.read_str(film_limits, "extension-value")?;
        previous_extension = Some(name);
        extensions.push(FilmExtensionView { name, value });
    }

    Ok(FilmRecordView {
        address,
        kind,
        datatype,
        identity,
        value,
        origin,
        span,
        extensions,
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
    let body_size =
        encoded_descriptor_body_size(descriptor, depth, film_limits, aes_limits, record_index)?;
    push_uleb(output, usize_to_u64(body_size, output.len(), record_index)?);
    let body_start = output.len();
    push_string(
        output,
        &descriptor.datatype,
        film_limits,
        "datatype-name",
        Some(record_index),
    )?;
    push_uleb(
        output,
        usize_to_u64(descriptor.generics.len(), output.len(), record_index)?,
    );
    for generic in &descriptor.generics {
        match generic {
            GenericArgument::Datatype(nested) => {
                output.push(0x00);
                push_descriptor(
                    output,
                    nested,
                    depth.saturating_add(1),
                    film_limits,
                    aes_limits,
                    record_index,
                )?;
            }
            GenericArgument::NumberLiteral(value) => {
                output.push(0x02);
                push_string(
                    output,
                    value,
                    film_limits,
                    "generic-number",
                    Some(record_index),
                )?;
            }
        }
    }
    push_uleb(
        output,
        usize_to_u64(descriptor.clarifiers.len(), output.len(), record_index)?,
    );
    for clarifier in &descriptor.clarifiers {
        output.push(match clarifier.kind {
            ClarifierKind::StringLiteral => 0x01,
            ClarifierKind::NumberLiteral => 0x02,
        });
        push_string(
            output,
            &clarifier.value,
            film_limits,
            "clarifier-value",
            Some(record_index),
        )?;
    }
    debug_assert_eq!(output.len().saturating_sub(body_start), body_size);
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
        "path",
        "header",
        "kind",
        "datatype",
        "generics",
        "clarifiers",
        "identity",
        "value",
        "origin",
        "span",
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

fn aes_limit_error(
    counter: &'static str,
    observed: usize,
    limit: usize,
    offset: usize,
    record: Option<usize>,
    component: &'static str,
) -> FilmError {
    FilmError {
        code: "FILM_AES_INVALID",
        offset,
        record,
        component,
        detail: format!("{counter} observed {observed}, limit {limit}"),
        diagnostics: vec![limit_diagnostic(counter, observed, limit)],
    }
}

fn claim_datatype_component(
    components: &mut usize,
    limit: usize,
    offset: usize,
    record: Option<usize>,
    component: &'static str,
) -> Result<(), FilmError> {
    let observed = components.checked_add(1).ok_or_else(|| {
        aes_limit_error(
            "max_datatype_components",
            limit.saturating_add(1),
            limit,
            offset,
            record,
            component,
        )
    })?;
    if observed > limit {
        return Err(aes_limit_error(
            "max_datatype_components",
            observed,
            limit,
            offset,
            record,
            component,
        ));
    }
    *components = observed;
    Ok(())
}

fn ensure_datatype_component_capacity(
    components: usize,
    additional: usize,
    limit: usize,
    offset: usize,
    record: Option<usize>,
    component: &'static str,
) -> Result<(), FilmError> {
    if additional > limit.saturating_sub(components) {
        return Err(aes_limit_error(
            "max_datatype_components",
            limit.saturating_add(1),
            limit,
            offset,
            record,
            component,
        ));
    }
    Ok(())
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

fn decode_vec_with_capacity<T>(
    capacity: usize,
    offset: usize,
    record: Option<usize>,
    component: &'static str,
) -> Result<Vec<T>, FilmError> {
    let mut output = Vec::new();
    output.try_reserve_exact(capacity).map_err(|_| {
        film_error(
            "FILM_INTEGER_OVERFLOW",
            offset,
            record,
            component,
            "Film collection cannot be allocated in the host size domain",
        )
    })?;
    Ok(output)
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

    fn read_limited_length(
        &mut self,
        counter: &'static str,
        limit: usize,
        component: &'static str,
        limit_component: &'static str,
    ) -> Result<usize, FilmError> {
        let offset = self.absolute_position();
        let value = self.read_uleb(component)?;
        let selected = u64::try_from(limit).unwrap_or(u64::MAX);
        if value > selected {
            return Err(film_error(
                "FILM_LIMIT_EXCEEDED",
                offset,
                self.record,
                limit_component,
                format!("{counter} observed {value}, limit {limit}"),
            ));
        }
        let length = usize::try_from(value).map_err(|_| {
            film_error(
                "FILM_INTEGER_OVERFLOW",
                offset,
                self.record,
                component,
                "Film length does not fit the host address space",
            )
        })?;
        Ok(length)
    }

    fn read_count(
        &mut self,
        counter: &'static str,
        limit: usize,
        component: &'static str,
    ) -> Result<usize, FilmError> {
        let offset = self.absolute_position();
        let value = self.read_uleb(component)?;
        let selected = u64::try_from(limit).unwrap_or(u64::MAX);
        if value > selected {
            return Err(aes_limit_error(
                counter,
                limit.saturating_add(1),
                limit,
                offset,
                self.record,
                component,
            ));
        }
        usize::try_from(value).map_err(|_| {
            film_error(
                "FILM_INTEGER_OVERFLOW",
                offset,
                self.record,
                component,
                "Film count does not fit the host address space",
            )
        })
    }

    fn read_str(
        &mut self,
        limits: &FilmLimits,
        component: &'static str,
    ) -> Result<&'a str, FilmError> {
        let offset = self.absolute_position();
        let length = self.read_limited_length(
            "max_field_bytes",
            limits.max_field_bytes,
            component,
            component,
        )?;
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
        Ok(value)
    }

    fn read_nonempty_context_str(
        &mut self,
        limits: &FilmLimits,
        component: &'static str,
    ) -> Result<&'a str, FilmError> {
        let offset = self.absolute_position();
        let value = self.read_str(limits, component)?;
        if value.is_empty() {
            return Err(film_error(
                "FILM_INVALID_CONTEXT",
                offset,
                self.record,
                component,
                "Film stream-context strings must not be empty",
            ));
        }
        Ok(value)
    }

    fn read_descriptor_view(
        &mut self,
        film_limits: &FilmLimits,
        aes_limits: &TelexLimits,
        depth: usize,
        components: &mut usize,
    ) -> Result<FilmDatatypeView<'a>, FilmError> {
        claim_datatype_component(
            components,
            aes_limits.max_datatype_components,
            self.absolute_position(),
            self.record,
            "datatype",
        )?;
        if depth > aes_limits.max_generic_depth {
            return Err(aes_limit_error(
                "max_generic_depth",
                depth,
                aes_limits.max_generic_depth,
                self.absolute_position(),
                self.record,
                "datatype",
            ));
        }
        let length = self.read_limited_length(
            "max_field_bytes",
            film_limits.max_field_bytes,
            "datatype",
            "datatype",
        )?;
        let offset = self.absolute_position();
        let bytes = self.read_exact(length, "datatype")?;
        let mut descriptor = Reader::new(bytes, offset, self.record);
        let datatype_offset = descriptor.absolute_position();
        let datatype = descriptor.read_str(film_limits, "datatype-name")?;
        if datatype.is_empty() {
            return Err(film_error(
                "FILM_INVALID_DATATYPE",
                datatype_offset,
                self.record,
                "datatype-name",
                "Film datatype names must not be empty",
            ));
        }
        let generic_count = descriptor.read_count(
            "max_generic_arguments",
            aes_limits.max_generic_arguments,
            "generic-count",
        )?;
        ensure_datatype_component_capacity(
            *components,
            generic_count,
            aes_limits.max_datatype_components,
            descriptor.absolute_position(),
            self.record,
            "generic-count",
        )?;
        let mut generics = decode_vec_with_capacity(
            generic_count,
            descriptor.absolute_position(),
            self.record,
            "generic-count",
        )?;
        for _ in 0..generic_count {
            let tag_offset = descriptor.absolute_position();
            match descriptor.read_byte("generic-tag")? {
                0x00 => generics.push(FilmGenericView::Datatype(descriptor.read_descriptor_view(
                    film_limits,
                    aes_limits,
                    depth.saturating_add(1),
                    components,
                )?)),
                0x02 => {
                    claim_datatype_component(
                        components,
                        aes_limits.max_datatype_components,
                        descriptor.absolute_position(),
                        self.record,
                        "generic-number",
                    )?;
                    generics.push(FilmGenericView::NumberLiteral(
                        descriptor.read_str(film_limits, "generic-number")?,
                    ));
                }
                _ => {
                    return Err(film_error(
                        "FILM_INVALID_DATATYPE",
                        tag_offset,
                        self.record,
                        "generic-tag",
                        "Film generic tag must be 00 or 02",
                    ));
                }
            }
        }
        let clarifier_count = descriptor.read_count(
            "max_clarifier_values",
            aes_limits.max_clarifier_values,
            "clarifier-count",
        )?;
        ensure_datatype_component_capacity(
            *components,
            clarifier_count,
            aes_limits.max_datatype_components,
            descriptor.absolute_position(),
            self.record,
            "clarifier-count",
        )?;
        let mut clarifiers = decode_vec_with_capacity(
            clarifier_count,
            descriptor.absolute_position(),
            self.record,
            "clarifier-count",
        )?;
        for _ in 0..clarifier_count {
            claim_datatype_component(
                components,
                aes_limits.max_datatype_components,
                descriptor.absolute_position(),
                self.record,
                "clarifier",
            )?;
            let tag_offset = descriptor.absolute_position();
            let kind = match descriptor.read_byte("clarifier-tag")? {
                0x01 => ClarifierKind::StringLiteral,
                0x02 => ClarifierKind::NumberLiteral,
                _ => {
                    return Err(film_error(
                        "FILM_INVALID_DATATYPE",
                        tag_offset,
                        self.record,
                        "clarifier-tag",
                        "Film clarifier tag must be 01 or 02",
                    ));
                }
            };
            clarifiers.push(FilmClarifierView {
                kind,
                value: descriptor.read_str(film_limits, "clarifier-value")?,
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
        Ok(FilmDatatypeView {
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
