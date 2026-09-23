//! Experimental checkpointed Film comparator.
//!
//! Candidate C retains Candidate B's UTF-8 byte-prefix delta but tags the
//! prefix token and resets both address planes every 256 records. The first
//! body and header address used in each block is absolute. Candidate C uses a
//! distinct preamble and is not Film v1.

use crate::film::{
    FilmError, FilmLimits, FilmRecordPrefix, FilmRecordView, FilmStream, decode_record_tail_view,
    kind_name,
};
use crate::film_candidate_b::encode_film_candidate_b_with_limits;
use crate::{
    COMPLETE_AES_PROFILE, CanonicalPathEvidence, ClarifierKind, DatatypeClarifier,
    DatatypeDescriptor, GenericArgument, PathDetails, PathDetailsSlice, RecordView, TelexLimits,
    TelexRecord, canonical_path_fingerprint, parse_canonical_data_path,
    validate_record_views_with_projection_and_limits,
    validate_record_views_with_projection_and_limits_path_arena,
};

pub const FILM_CANDIDATE_C_PREAMBLE: [u8; 5] = [0x4f, 0x5f, 0x43, 0xff, 0x00];
pub const FILM_CANDIDATE_C_CHECKPOINT_INTERVAL: usize = 256;

const FILM_CANDIDATE_B_PREAMBLE: [u8; 5] = [0x4f, 0x5f, 0x42, 0xff, 0x00];
const HEADER_PLANE: u8 = 0x01;
const ABSENT_INDEX: u32 = u32::MAX;
const ABSENT_TEXT: CompactText = CompactText {
    start: 0,
    len: u32::MAX,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateCCheckpoint {
    pub record_index: usize,
    pub byte_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateCIndex {
    pub context_end: usize,
    pub input_bytes: usize,
    pub checkpoints: Vec<CandidateCCheckpoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactText {
    start: u32,
    len: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompactGeneric {
    Datatype(u32),
    NumberLiteral(CompactText),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactClarifier {
    pub kind: ClarifierKind,
    pub value: CompactText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactDatatype {
    pub name: CompactText,
    pub generic_start: u32,
    pub generic_count: u32,
    pub clarifier_start: u32,
    pub clarifier_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactExtension {
    pub name: CompactText,
    pub value: CompactText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactFilmRecord {
    address: CompactText,
    value: CompactText,
    metadata_index: u32,
    kind_code: u8,
    is_header: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompactRecordMetadata {
    datatype: Option<u32>,
    identity: Option<CompactText>,
    origin: Option<[u8; 32]>,
    span: Option<(u64, u64)>,
    extension_start: u32,
    extension_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompactPathEvidence {
    details: PathDetails,
    fingerprint: u64,
}

struct CompactValidationContext<'a> {
    stream: &'a CompactFilmStream,
    datatypes: Vec<DatatypeDescriptor>,
    formatted: Vec<FormattedMetadata>,
}

struct FormattedMetadata {
    origin: Option<String>,
    span: Option<String>,
}

#[derive(Clone, Copy)]
struct PreparedValidationRecord {
    datatype_index: u32,
    formatted_index: u32,
}

struct CompactValidationRecord<'a> {
    context: &'a CompactValidationContext<'a>,
    record_index: u32,
    prepared: PreparedValidationRecord,
}

impl RecordView for CompactValidationRecord<'_> {
    const FIXED_CORE_FIELDS: bool = true;

    fn field_count(&self) -> usize {
        let record = self.record();
        let metadata = self.context.stream.record_metadata(record);
        2 + usize::from(record.value != ABSENT_TEXT)
            + metadata.map_or(0, |metadata| {
                usize::from(metadata.datatype.is_some())
                    + usize::from(metadata.identity.is_some())
                    + usize::from(metadata.origin.is_some())
                    + usize::from(metadata.span.is_some())
                    + metadata.extension_count as usize
            })
    }

    fn get(&self, field: &str) -> Option<&str> {
        let stream = self.context.stream;
        let record = self.record();
        let metadata = stream.record_metadata(record);
        match field {
            "header" if record.is_header => Some(stream.text(record.address)),
            "path" if !record.is_header => Some(stream.text(record.address)),
            "kind" => kind_name(record.kind_code),
            "datatype" => self.datatype().map(|value| value.datatype.as_str()),
            "identity" => metadata
                .and_then(|value| value.identity)
                .map(|value| stream.text(value)),
            "value" => (record.value != ABSENT_TEXT).then(|| stream.text(record.value)),
            "origin" => self.formatted().and_then(|value| value.origin.as_deref()),
            "span" => self.formatted().and_then(|value| value.span.as_deref()),
            extension_name => stream
                .record_extensions(record)
                .iter()
                .find(|extension| stream.text(extension.name) == extension_name)
                .map(|extension| stream.text(extension.value)),
        }
    }

    fn datatype(&self) -> Option<&DatatypeDescriptor> {
        (self.prepared.datatype_index != ABSENT_INDEX)
            .then(|| &self.context.datatypes[self.prepared.datatype_index as usize])
    }

    fn canonical_path_evidence(&self) -> Option<CanonicalPathEvidence<'_>> {
        self.context
            .stream
            .path_evidence
            .get(self.record_index as usize)
            .and_then(Option::as_ref)
            .map(|evidence| CanonicalPathEvidence {
                details: PathDetailsSlice::new(&evidence.details.segments),
                fingerprint: evidence.fingerprint,
            })
    }

    fn visit_fields<'a>(&'a self, visitor: &mut dyn FnMut(&'a str, &'a str)) {
        let stream = self.context.stream;
        let record = self.record();
        visitor(
            if record.is_header { "header" } else { "path" },
            stream.text(record.address),
        );
        visitor(
            "kind",
            kind_name(record.kind_code).expect("compact Film kind was validated"),
        );
        if let Some(datatype) = self.datatype() {
            visitor("datatype", &datatype.datatype);
        }
        if let Some(identity) = stream.record_identity(record) {
            visitor("identity", identity);
        }
        if record.value != ABSENT_TEXT {
            visitor("value", stream.text(record.value));
        }
        if let Some(origin) = self.formatted().and_then(|value| value.origin.as_deref()) {
            visitor("origin", origin);
        }
        if let Some(span) = self.formatted().and_then(|value| value.span.as_deref()) {
            visitor("span", span);
        }
        for extension in stream.record_extensions(record) {
            visitor(stream.text(extension.name), stream.text(extension.value));
        }
    }
}

impl CompactValidationRecord<'_> {
    fn record(&self) -> &CompactFilmRecord {
        &self.context.stream.records[self.record_index as usize]
    }

    fn formatted(&self) -> Option<&FormattedMetadata> {
        (self.prepared.formatted_index != ABSENT_INDEX)
            .then(|| &self.context.formatted[self.prepared.formatted_index as usize])
    }
}

/// General typed owned result for the Candidate C experiment. Variable text
/// lives in one byte slab; recursive datatypes, generics, clarifiers, and
/// extensions use typed side vectors instead of textual `TelexRecord` pairs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactFilmStream {
    profile: CompactText,
    profile_explicit: bool,
    projection: Option<CompactText>,
    projection_explicit: bool,
    records: Vec<CompactFilmRecord>,
    datatypes: Vec<CompactDatatype>,
    generics: Vec<CompactGeneric>,
    clarifiers: Vec<CompactClarifier>,
    extensions: Vec<CompactExtension>,
    metadata: Vec<CompactRecordMetadata>,
    path_evidence: Vec<Option<CompactPathEvidence>>,
    bytes: Vec<u8>,
    input_bytes: usize,
}

impl CompactFilmStream {
    #[must_use]
    pub fn profile(&self) -> &str {
        self.text(self.profile)
    }

    #[must_use]
    pub fn profile_explicit(&self) -> bool {
        self.profile_explicit
    }

    #[must_use]
    pub fn projection(&self) -> Option<&str> {
        self.projection.map(|value| self.text(value))
    }

    #[must_use]
    pub fn projection_explicit(&self) -> bool {
        self.projection_explicit
    }

    #[must_use]
    pub fn records(&self) -> &[CompactFilmRecord] {
        &self.records
    }

    #[must_use]
    pub fn record_is_header(&self, record: &CompactFilmRecord) -> bool {
        record.is_header
    }

    #[must_use]
    pub fn record_address<'a>(&'a self, record: &CompactFilmRecord) -> &'a str {
        self.text(record.address)
    }

    #[must_use]
    pub fn record_kind(&self, record: &CompactFilmRecord) -> &'static str {
        kind_name(record.kind_code).expect("compact Film kind was validated")
    }

    #[must_use]
    pub fn record_value<'a>(&'a self, record: &CompactFilmRecord) -> Option<&'a str> {
        (record.value != ABSENT_TEXT).then(|| self.text(record.value))
    }

    #[must_use]
    pub fn record_datatype(&self, record: &CompactFilmRecord) -> Option<&CompactDatatype> {
        self.record_metadata(record)
            .and_then(|metadata| metadata.datatype)
            .map(|index| &self.datatypes[index as usize])
    }

    #[must_use]
    pub fn record_identity<'a>(&'a self, record: &CompactFilmRecord) -> Option<&'a str> {
        self.record_metadata(record)
            .and_then(|metadata| metadata.identity)
            .map(|value| self.text(value))
    }

    #[must_use]
    pub fn record_origin(&self, record: &CompactFilmRecord) -> Option<&[u8; 32]> {
        self.record_metadata(record)
            .and_then(|metadata| metadata.origin.as_ref())
    }

    #[must_use]
    pub fn record_span(&self, record: &CompactFilmRecord) -> Option<(u64, u64)> {
        self.record_metadata(record)
            .and_then(|metadata| metadata.span)
    }

    #[must_use]
    pub fn record_extensions(&self, record: &CompactFilmRecord) -> &[CompactExtension] {
        let Some(metadata) = self.record_metadata(record) else {
            return &[];
        };
        let start = metadata.extension_start as usize;
        let end = start + metadata.extension_count as usize;
        &self.extensions[start..end]
    }

    #[must_use]
    pub fn datatypes(&self) -> &[CompactDatatype] {
        &self.datatypes
    }

    #[must_use]
    pub fn generics(&self) -> &[CompactGeneric] {
        &self.generics
    }

    #[must_use]
    pub fn clarifiers(&self) -> &[CompactClarifier] {
        &self.clarifiers
    }

    #[must_use]
    pub fn text(&self, value: CompactText) -> &str {
        assert_ne!(value.len, u32::MAX, "absent compact text has no value");
        let start = value.start as usize;
        let end = start + value.len as usize;
        // CompactText values are created only after UTF-8 validation.
        std::str::from_utf8(&self.bytes[start..end]).expect("validated compact Film text")
    }

    #[must_use]
    pub fn storage_bytes(&self) -> usize {
        self.bytes.len()
            + self.records.len() * std::mem::size_of::<CompactFilmRecord>()
            + self.datatypes.len() * std::mem::size_of::<CompactDatatype>()
            + self.generics.len() * std::mem::size_of::<CompactGeneric>()
            + self.clarifiers.len() * std::mem::size_of::<CompactClarifier>()
            + self.extensions.len() * std::mem::size_of::<CompactExtension>()
            + self.metadata.len() * std::mem::size_of::<CompactRecordMetadata>()
            + self.path_evidence_bytes()
    }

    #[must_use]
    pub fn cached_path_count(&self) -> usize {
        self.path_evidence.iter().flatten().count()
    }

    #[must_use]
    pub fn path_evidence_bytes(&self) -> usize {
        self.path_evidence.len() * std::mem::size_of::<Option<CompactPathEvidence>>()
            + self
                .path_evidence
                .iter()
                .flatten()
                .map(|evidence| {
                    evidence.details.segments.len() * std::mem::size_of::<crate::ParsedSegment>()
                })
                .sum::<usize>()
    }

    #[must_use]
    pub fn input_bytes(&self) -> usize {
        self.input_bytes
    }

    #[must_use]
    pub fn to_owned_unvalidated(&self) -> FilmStream {
        FilmStream {
            profile: self.profile().to_owned(),
            profile_explicit: self.profile_explicit,
            projection: self.projection().map(str::to_owned),
            projection_explicit: self.projection_explicit,
            records: self
                .records
                .iter()
                .map(|record| self.record_to_owned(record))
                .collect(),
        }
    }

    fn record_to_owned(&self, record: &CompactFilmRecord) -> TelexRecord {
        let metadata = (record.metadata_index != ABSENT_INDEX)
            .then(|| &self.metadata[record.metadata_index as usize]);
        let mut fields = vec![
            (
                if record.is_header { "header" } else { "path" }.to_owned(),
                self.text(record.address).to_owned(),
            ),
            (
                "kind".to_owned(),
                kind_name(record.kind_code)
                    .expect("compact Film kind was validated")
                    .to_owned(),
            ),
        ];
        let datatype = metadata
            .and_then(|metadata| metadata.datatype)
            .map(|index| self.datatype_to_owned(index as usize));
        if let Some(descriptor) = &datatype {
            fields.push(("datatype".to_owned(), descriptor.datatype.clone()));
        }
        if let Some(identity) = metadata.and_then(|metadata| metadata.identity) {
            fields.push(("identity".to_owned(), self.text(identity).to_owned()));
        }
        if record.value != ABSENT_TEXT {
            fields.push(("value".to_owned(), self.text(record.value).to_owned()));
        }
        if let Some(origin) = metadata.and_then(|metadata| metadata.origin) {
            fields.push(("origin".to_owned(), encode_origin(&origin)));
        }
        if let Some((start, end)) = metadata.and_then(|metadata| metadata.span) {
            fields.push(("span".to_owned(), format!("{start}:{end}")));
        }
        if let Some(metadata) = metadata {
            let extension_start = metadata.extension_start as usize;
            let extension_end = extension_start + metadata.extension_count as usize;
            fields.extend(self.extensions[extension_start..extension_end].iter().map(
                |extension| {
                    (
                        self.text(extension.name).to_owned(),
                        self.text(extension.value).to_owned(),
                    )
                },
            ));
        }
        match datatype {
            Some(descriptor) => TelexRecord::with_datatype(fields, descriptor),
            None => TelexRecord::new(fields),
        }
    }

    fn datatype_to_owned(&self, index: usize) -> DatatypeDescriptor {
        let datatype = self.datatypes[index];
        let generic_start = datatype.generic_start as usize;
        let generic_end = generic_start + datatype.generic_count as usize;
        let clarifier_start = datatype.clarifier_start as usize;
        let clarifier_end = clarifier_start + datatype.clarifier_count as usize;
        DatatypeDescriptor {
            datatype: self.text(datatype.name).to_owned(),
            generics: self.generics[generic_start..generic_end]
                .iter()
                .map(|generic| match generic {
                    CompactGeneric::Datatype(index) => {
                        GenericArgument::Datatype(self.datatype_to_owned(*index as usize))
                    }
                    CompactGeneric::NumberLiteral(value) => {
                        GenericArgument::NumberLiteral(self.text(*value).to_owned())
                    }
                })
                .collect(),
            clarifiers: self.clarifiers[clarifier_start..clarifier_end]
                .iter()
                .map(|clarifier| DatatypeClarifier {
                    kind: clarifier.kind.clone(),
                    value: self.text(clarifier.value).to_owned(),
                })
                .collect(),
        }
    }

    fn record_metadata(&self, record: &CompactFilmRecord) -> Option<&CompactRecordMetadata> {
        (record.metadata_index != ABSENT_INDEX)
            .then(|| &self.metadata[record.metadata_index as usize])
    }
}

pub fn encode_film_candidate_c(
    stream: &FilmStream,
    registered_fields: &[&str],
) -> Result<Vec<u8>, FilmError> {
    encode_film_candidate_c_with_limits(
        stream,
        registered_fields,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
}

pub fn encode_film_candidate_c_with_limits(
    stream: &FilmStream,
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<Vec<u8>, FilmError> {
    let candidate_b =
        encode_film_candidate_b_with_limits(stream, registered_fields, film_limits, aes_limits)?;
    candidate_b_to_c(&candidate_b, film_limits)
}

pub fn decode_film_candidate_c(
    input: &[u8],
    registered_fields: &[&str],
) -> Result<FilmStream, FilmError> {
    decode_film_candidate_c_with_limits(
        input,
        registered_fields,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
}

pub fn decode_film_candidate_c_with_limits(
    input: &[u8],
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<FilmStream, FilmError> {
    let compact = decode_film_candidate_c_validated_compact_with_limits(
        input,
        registered_fields,
        film_limits,
        aes_limits,
    )?;
    Ok(compact.to_owned_unvalidated())
}

pub fn decode_film_candidate_c_validated_compact(
    input: &[u8],
    registered_fields: &[&str],
) -> Result<CompactFilmStream, FilmError> {
    decode_film_candidate_c_validated_compact_with_limits(
        input,
        registered_fields,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
}

pub fn decode_film_candidate_c_validated_compact_with_limits(
    input: &[u8],
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<CompactFilmStream, FilmError> {
    let compact = decode_film_candidate_c_compact_with_limits(input, film_limits, aes_limits)?;
    validate_film_candidate_c_compact(&compact, registered_fields, aes_limits)?;
    Ok(compact)
}

/// Experimental completion path that parses canonical paths into one
/// transient segment arena. The arena is discarded after portable AES
/// validation and is not retained by the compact result.
pub fn decode_film_candidate_c_validated_compact_path_arena_with_limits(
    input: &[u8],
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<CompactFilmStream, FilmError> {
    let compact = decode_film_candidate_c_compact_with_limits(input, film_limits, aes_limits)?;
    validate_film_candidate_c_compact_internal(&compact, registered_fields, aes_limits, true)?;
    Ok(compact)
}

/// Experimental completion path that retains parsed canonical path evidence
/// from physical decoding for reuse by portable AES validation.
pub fn decode_film_candidate_c_validated_compact_cached_paths_with_limits(
    input: &[u8],
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<CompactFilmStream, FilmError> {
    let compact =
        decode_film_candidate_c_compact_cached_paths_with_limits(input, film_limits, aes_limits)?;
    validate_film_candidate_c_compact(&compact, registered_fields, aes_limits)?;
    Ok(compact)
}

pub fn validate_film_candidate_c_compact(
    stream: &CompactFilmStream,
    registered_fields: &[&str],
    limits: &TelexLimits,
) -> Result<(), FilmError> {
    validate_film_candidate_c_compact_internal(stream, registered_fields, limits, false)
}

/// Experimental portable-validation adapter backed by a transient path arena.
/// The compact stream is not modified and retains no path evidence.
pub fn validate_film_candidate_c_compact_path_arena(
    stream: &CompactFilmStream,
    registered_fields: &[&str],
    limits: &TelexLimits,
) -> Result<(), FilmError> {
    validate_film_candidate_c_compact_internal(stream, registered_fields, limits, true)
}

fn validate_film_candidate_c_compact_internal(
    stream: &CompactFilmStream,
    registered_fields: &[&str],
    limits: &TelexLimits,
    use_transient_path_arena: bool,
) -> Result<(), FilmError> {
    let mut datatypes = Vec::new();
    let mut formatted = Vec::new();
    let mut prepared = Vec::with_capacity(stream.records.len());
    for record in &stream.records {
        let metadata = stream.record_metadata(record);
        let datatype_index = if let Some(index) = metadata.and_then(|value| value.datatype) {
            let prepared_index = u32_index(datatypes.len(), None, "validation-datatypes")?;
            datatypes.push(stream.datatype_to_owned(index as usize));
            prepared_index
        } else {
            ABSENT_INDEX
        };
        let formatted_index =
            if metadata.is_some_and(|value| value.origin.is_some() || value.span.is_some()) {
                let prepared_index = u32_index(formatted.len(), None, "validation-metadata")?;
                formatted.push(FormattedMetadata {
                    origin: metadata
                        .and_then(|value| value.origin)
                        .map(|value| encode_origin(&value)),
                    span: metadata
                        .and_then(|value| value.span)
                        .map(|(start, end)| format!("{start}:{end}")),
                });
                prepared_index
            } else {
                ABSENT_INDEX
            };
        prepared.push(PreparedValidationRecord {
            datatype_index,
            formatted_index,
        });
    }
    let context = CompactValidationContext {
        stream,
        datatypes,
        formatted,
    };
    let records = prepared
        .iter()
        .copied()
        .enumerate()
        .map(|(record_index, prepared)| {
            Ok(CompactValidationRecord {
                context: &context,
                record_index: u32_index(record_index, Some(record_index), "validation-record")?,
                prepared,
            })
        })
        .collect::<Result<Vec<_>, FilmError>>()?;
    let validation = if use_transient_path_arena {
        validate_record_views_with_projection_and_limits_path_arena(
            &records,
            stream.profile(),
            stream.projection(),
            registered_fields,
            limits,
        )
    } else {
        validate_record_views_with_projection_and_limits(
            &records,
            stream.profile(),
            stream.projection(),
            registered_fields,
            limits,
        )
    };
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
        offset: stream.input_bytes,
        record,
        component: "aes-events",
        detail,
        diagnostics: validation.diagnostics,
    })
}

pub fn decode_film_candidate_c_compact(input: &[u8]) -> Result<CompactFilmStream, FilmError> {
    decode_film_candidate_c_compact_with_limits(
        input,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
}

pub fn decode_film_candidate_c_compact_with_limits(
    input: &[u8],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<CompactFilmStream, FilmError> {
    decode_film_candidate_c_compact_internal(input, film_limits, aes_limits, false)
}

/// Experimental physical decode that retains parsed canonical path evidence.
pub fn decode_film_candidate_c_compact_cached_paths_with_limits(
    input: &[u8],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<CompactFilmStream, FilmError> {
    decode_film_candidate_c_compact_internal(input, film_limits, aes_limits, true)
}

fn decode_film_candidate_c_compact_internal(
    input: &[u8],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
    retain_path_evidence: bool,
) -> Result<CompactFilmStream, FilmError> {
    check_limit(
        "max_input_bytes",
        input.len(),
        film_limits.max_input_bytes,
        0,
        None,
        "stream",
    )?;
    let mut reader = Cursor::new(input);
    reader.expect_preamble(&FILM_CANDIDATE_C_PREAMBLE, "candidate-c-preamble")?;
    let context = reader.read_byte("stream-context")?;
    if context & !0x03 != 0 {
        return Err(error(
            "FILM_COMPARATOR_NONCANONICAL",
            reader.absolute_position().saturating_sub(1),
            None,
            "stream-context",
            "Reserved context bits must be zero",
        ));
    }
    let profile_explicit = context & 0x01 != 0;
    let projection_explicit = context & 0x02 != 0;
    let mut builder = CompactBuilder::new(
        input.len(),
        profile_explicit,
        projection_explicit,
        aes_limits.max_decoded_payload_bytes,
        retain_path_evidence,
    );
    builder.profile = if profile_explicit {
        let value = read_nonempty_context(&mut reader, film_limits, "profile")?;
        builder.push_text(value, None, "profile")?
    } else {
        builder.push_text(COMPLETE_AES_PROFILE, None, "profile")?
    };
    builder.projection = if projection_explicit {
        let value = read_nonempty_context(&mut reader, film_limits, "projection")?;
        Some(builder.push_text(value, None, "projection")?)
    } else {
        None
    };

    let mut previous_body = Vec::new();
    let mut previous_header = Vec::new();
    let mut body_initialized = false;
    let mut header_initialized = false;
    while !reader.is_empty() {
        let record_index = builder.records.len();
        if record_index.is_multiple_of(FILM_CANDIDATE_C_CHECKPOINT_INTERVAL) {
            previous_body.clear();
            previous_header.clear();
            body_initialized = false;
            header_initialized = false;
        }
        if record_index >= aes_limits.max_events {
            return Err(error(
                "FILM_LIMIT_EXCEEDED",
                reader.absolute_position(),
                Some(record_index),
                "record",
                format!(
                    "max_events observed {}, limit {}",
                    record_index.saturating_add(1),
                    aes_limits.max_events
                ),
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
            return Err(error(
                "FILM_COMPARATOR_NONCANONICAL",
                reader.absolute_position(),
                Some(record_index),
                "record-length",
                "Candidate C record length must be positive",
            ));
        }
        let payload_offset = reader.absolute_position();
        let payload = reader.read_exact(payload_length, "record")?;
        let mut record = Cursor::with_base(payload, payload_offset, Some(record_index));
        let control_offset = record.absolute_position();
        let control = record.read_byte("record-control")?;
        let kind_offset = record.absolute_position();
        let kind_code = record.read_byte("kind")?;
        let token_offset = record.absolute_position();
        let token = record.read_uleb("address-token")?;
        let checkpoint = token & 1 == 1;
        let prefix_value = token >> 1;
        let suffix = record.read_string_bytes(film_limits, "address-suffix")?;
        let (previous, plane_initialized) = if control & HEADER_PLANE == 0 {
            (&mut previous_body, &mut body_initialized)
        } else {
            (&mut previous_header, &mut header_initialized)
        };
        if checkpoint == *plane_initialized {
            return Err(error(
                "FILM_CHECKPOINT_SCHEDULE",
                token_offset,
                Some(record_index),
                "address-token",
                format!(
                    "Candidate C requires the first address in each plane of every {}-record block to be absolute",
                    FILM_CANDIDATE_C_CHECKPOINT_INTERVAL
                ),
            ));
        }
        if checkpoint && token != 1 {
            return Err(error(
                "FILM_COMPARATOR_NONCANONICAL",
                token_offset,
                Some(record_index),
                "address-token",
                "An absolute checkpoint token cannot retain a prefix",
            ));
        }
        let prefix_length = usize::try_from(prefix_value).map_err(|_| {
            error(
                "FILM_INTEGER_OVERFLOW",
                token_offset,
                Some(record_index),
                "address-token",
                "Address prefix length exceeds the host address space",
            )
        })?;
        if prefix_length > previous.len() || !is_char_boundary(previous, prefix_length) {
            return Err(error(
                "FILM_COMPARATOR_INVALID_PREFIX",
                token_offset,
                Some(record_index),
                "address-token",
                "Address prefix exceeds the previous address or splits UTF-8",
            ));
        }
        let address_length = prefix_length.checked_add(suffix.len()).ok_or_else(|| {
            error(
                "FILM_INTEGER_OVERFLOW",
                token_offset,
                Some(record_index),
                "address",
                "Reconstructed address length overflow",
            )
        })?;
        check_limit(
            "max_field_bytes",
            address_length,
            film_limits.max_field_bytes,
            token_offset,
            Some(record_index),
            "address",
        )?;
        check_limit(
            "max_buffered_bytes",
            address_length,
            film_limits.max_buffered_bytes,
            token_offset,
            Some(record_index),
            "previous-address",
        )?;
        let expanded_size = checked_size(
            checked_size(
                2,
                encoded_string_size(address_length, payload_offset, Some(record_index))?,
                payload_offset,
                Some(record_index),
                "expanded-record",
            )?,
            record.remaining().len(),
            payload_offset,
            Some(record_index),
            "expanded-record",
        )?;
        check_limit(
            "max_record_bytes",
            expanded_size,
            film_limits.max_record_bytes,
            payload_offset,
            Some(record_index),
            "expanded-record",
        )?;
        check_limit(
            "max_buffered_bytes",
            expanded_size,
            film_limits.max_buffered_bytes,
            payload_offset,
            Some(record_index),
            "expanded-record",
        )?;

        if !checkpoint {
            let additional_prefix = common_utf8_prefix_bytes(&previous[prefix_length..], suffix)?;
            if additional_prefix != 0 {
                return Err(error(
                    "FILM_COMPARATOR_NONCANONICAL",
                    token_offset,
                    Some(record_index),
                    "address-token",
                    "Candidate C requires the longest shared UTF-8 prefix between checkpoints",
                ));
            }
        }
        previous.truncate(prefix_length);
        previous.try_reserve(suffix.len()).map_err(|_| {
            error(
                "FILM_INTEGER_OVERFLOW",
                token_offset,
                Some(record_index),
                "address",
                "Reusable address buffer cannot grow in the host size domain",
            )
        })?;
        previous.extend_from_slice(suffix);
        debug_assert_eq!(previous.len(), address_length);
        let address_text = std::str::from_utf8(previous).map_err(|invalid| {
            error(
                "FILM_INVALID_UTF8",
                token_offset.saturating_add(invalid.valid_up_to()),
                Some(record_index),
                "address",
                "Reconstructed address is not UTF-8",
            )
        })?;
        let path_evidence = retain_path_evidence
            .then(|| {
                parse_canonical_data_path(address_text)
                    .ok()
                    .map(|details| CompactPathEvidence {
                        details,
                        fingerprint: canonical_path_fingerprint(address_text),
                    })
            })
            .flatten();

        let tail_offset = record.absolute_position();
        {
            let view = decode_record_tail_view(
                FilmRecordPrefix {
                    control,
                    control_offset,
                    kind_code,
                    kind_offset,
                    address_value: address_text,
                    tail_offset,
                },
                record.remaining(),
                record_index,
                film_limits,
                aes_limits,
            )?;
            builder.push_record(&view, kind_code, record_index, path_evidence)?;
        }
        *plane_initialized = true;
    }

    Ok(builder.finish())
}

/// Builds a lightweight directory of absolute checkpoint frame offsets. The
/// directory can be retained separately so a later damaged block does not
/// prevent access to subsequent blocks whose frame offsets are already known.
pub fn index_film_candidate_c(
    input: &[u8],
    film_limits: &FilmLimits,
) -> Result<CandidateCIndex, FilmError> {
    check_limit(
        "max_input_bytes",
        input.len(),
        film_limits.max_input_bytes,
        0,
        None,
        "stream",
    )?;
    let mut reader = Cursor::new(input);
    reader.expect_preamble(&FILM_CANDIDATE_C_PREAMBLE, "candidate-c-preamble")?;
    let context_end = read_context_end_validated(&mut reader, film_limits)?;
    let mut checkpoints = Vec::new();
    let mut record_index = 0_usize;
    let mut body_initialized = false;
    let mut header_initialized = false;
    while !reader.is_empty() {
        let block_start = record_index.is_multiple_of(FILM_CANDIDATE_C_CHECKPOINT_INTERVAL);
        if block_start {
            body_initialized = false;
            header_initialized = false;
        }
        let frame_offset = reader.absolute_position();
        reader.record = Some(record_index);
        let payload_length = reader.read_limited_length(
            "max_record_bytes",
            film_limits.max_record_bytes,
            "record-length",
            "record",
        )?;
        if payload_length == 0 {
            return Err(error(
                "FILM_COMPARATOR_NONCANONICAL",
                reader.absolute_position(),
                Some(record_index),
                "record-length",
                "Candidate C record length must be positive",
            ));
        }
        let payload_offset = reader.absolute_position();
        let payload = reader.read_exact(payload_length, "record")?;
        let mut record = Cursor::with_base(payload, payload_offset, Some(record_index));
        let control = record.read_byte("record-control")?;
        record.read_byte("kind")?;
        let token_offset = record.absolute_position();
        let token = record.read_uleb("address-token")?;
        let checkpoint = token & 1 == 1;
        let plane_initialized = if control & HEADER_PLANE == 0 {
            &mut body_initialized
        } else {
            &mut header_initialized
        };
        if checkpoint == *plane_initialized || (checkpoint && token != 1) {
            return Err(error(
                "FILM_CHECKPOINT_SCHEDULE",
                token_offset,
                Some(record_index),
                "address-token",
                "Candidate C checkpoint marker does not match the fixed schedule",
            ));
        }
        if block_start {
            checkpoints.push(CandidateCCheckpoint {
                record_index,
                byte_offset: frame_offset,
            });
        }
        *plane_initialized = true;
        record_index = record_index.saturating_add(1);
    }
    Ok(CandidateCIndex {
        context_end,
        input_bytes: input.len(),
        checkpoints,
    })
}

/// Physically decodes one independently addressable checkpoint block. This is
/// intentionally provisional: complete AES validation still requires the
/// complete logical stream.
pub fn decode_film_candidate_c_compact_block_with_limits(
    input: &[u8],
    index: &CandidateCIndex,
    block: usize,
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<CompactFilmStream, FilmError> {
    if index.input_bytes != input.len() || index.context_end > input.len() {
        return Err(error(
            "FILM_CHECKPOINT_INDEX_MISMATCH",
            0,
            None,
            "checkpoint-index",
            "Checkpoint directory does not match the input extent",
        ));
    }
    let checkpoint = index.checkpoints.get(block).ok_or_else(|| {
        error(
            "FILM_CHECKPOINT_INDEX_RANGE",
            0,
            None,
            "checkpoint-index",
            "Requested checkpoint block is outside the directory",
        )
    })?;
    let end = index
        .checkpoints
        .get(block.saturating_add(1))
        .map_or(input.len(), |next| next.byte_offset);
    if checkpoint.byte_offset < index.context_end
        || checkpoint.byte_offset >= end
        || end > input.len()
    {
        return Err(error(
            "FILM_CHECKPOINT_INDEX_MISMATCH",
            checkpoint.byte_offset,
            Some(checkpoint.record_index),
            "checkpoint-index",
            "Checkpoint directory contains an invalid byte range",
        ));
    }
    let capacity = checked_size(
        index.context_end,
        end - checkpoint.byte_offset,
        checkpoint.byte_offset,
        Some(checkpoint.record_index),
        "checkpoint-block",
    )?;
    let mut block_input = buffer_with_capacity(capacity, None, "checkpoint-block")?;
    block_input.extend_from_slice(&input[..index.context_end]);
    block_input.extend_from_slice(&input[checkpoint.byte_offset..end]);
    decode_film_candidate_c_compact_with_limits(&block_input, film_limits, aes_limits)
}

struct CompactBuilder {
    profile: CompactText,
    profile_explicit: bool,
    projection: Option<CompactText>,
    projection_explicit: bool,
    records: Vec<CompactFilmRecord>,
    datatypes: Vec<CompactDatatype>,
    generics: Vec<CompactGeneric>,
    clarifiers: Vec<CompactClarifier>,
    extensions: Vec<CompactExtension>,
    metadata: Vec<CompactRecordMetadata>,
    path_evidence: Vec<Option<CompactPathEvidence>>,
    bytes: Vec<u8>,
    input_bytes: usize,
    max_slab_bytes: usize,
    retain_path_evidence: bool,
}

impl CompactBuilder {
    fn new(
        input_bytes: usize,
        profile_explicit: bool,
        projection_explicit: bool,
        max_slab_bytes: usize,
        retain_path_evidence: bool,
    ) -> Self {
        Self {
            profile: CompactText { start: 0, len: 0 },
            profile_explicit,
            projection: None,
            projection_explicit,
            records: Vec::new(),
            datatypes: Vec::new(),
            generics: Vec::new(),
            clarifiers: Vec::new(),
            extensions: Vec::new(),
            metadata: Vec::new(),
            path_evidence: Vec::new(),
            bytes: Vec::with_capacity(input_bytes.min(max_slab_bytes)),
            input_bytes,
            max_slab_bytes,
            retain_path_evidence,
        }
    }

    fn push_text(
        &mut self,
        value: &str,
        record: Option<usize>,
        component: &'static str,
    ) -> Result<CompactText, FilmError> {
        let projected = self.bytes.len().checked_add(value.len()).ok_or_else(|| {
            error(
                "FILM_INTEGER_OVERFLOW",
                self.bytes.len(),
                record,
                component,
                "Compact text slab size overflow",
            )
        })?;
        check_limit(
            "max_decoded_payload_bytes",
            projected,
            self.max_slab_bytes,
            self.bytes.len(),
            record,
            component,
        )?;
        let start = u32::try_from(self.bytes.len()).map_err(|_| {
            error(
                "FILM_INTEGER_OVERFLOW",
                self.bytes.len(),
                record,
                component,
                "Compact text slab exceeds u32 offsets",
            )
        })?;
        let len = u32::try_from(value.len()).map_err(|_| {
            error(
                "FILM_INTEGER_OVERFLOW",
                self.bytes.len(),
                record,
                component,
                "Compact text length exceeds u32",
            )
        })?;
        if start == ABSENT_INDEX || len == u32::MAX {
            return Err(error(
                "FILM_INTEGER_OVERFLOW",
                self.bytes.len(),
                record,
                component,
                "Compact text cannot use the reserved absent sentinel",
            ));
        }
        self.bytes.try_reserve(value.len()).map_err(|_| {
            error(
                "FILM_INTEGER_OVERFLOW",
                self.bytes.len(),
                record,
                component,
                "Compact text slab cannot grow in the host size domain",
            )
        })?;
        self.bytes.extend_from_slice(value.as_bytes());
        Ok(CompactText { start, len })
    }

    fn push_record(
        &mut self,
        view: &FilmRecordView<'_>,
        kind_code: u8,
        record_index: usize,
        path_evidence: Option<CompactPathEvidence>,
    ) -> Result<(), FilmError> {
        let address = self.push_text(view.address.value(), Some(record_index), "address")?;
        let datatype = view
            .datatype
            .as_ref()
            .map(|value| self.push_datatype(value, record_index))
            .transpose()?;
        let identity = view
            .identity
            .map(|value| self.push_text(value, Some(record_index), "identity"))
            .transpose()?;
        let value = view
            .value
            .map(|value| self.push_text(value, Some(record_index), "value"))
            .transpose()?
            .unwrap_or(ABSENT_TEXT);
        let extension_start = u32_index(self.extensions.len(), Some(record_index), "extensions")?;
        for extension in &view.extensions {
            let name = self.push_text(extension.name, Some(record_index), "extension-name")?;
            let value = self.push_text(extension.value, Some(record_index), "extension-value")?;
            self.extensions.push(CompactExtension { name, value });
        }
        let extension_count = u32_index(view.extensions.len(), Some(record_index), "extensions")?;
        let has_metadata = datatype.is_some()
            || identity.is_some()
            || view.origin.is_some()
            || view.span.is_some()
            || extension_count != 0;
        let metadata_index = if has_metadata {
            let index = u32_index(self.metadata.len(), Some(record_index), "record-metadata")?;
            if index == ABSENT_INDEX {
                return Err(error(
                    "FILM_INTEGER_OVERFLOW",
                    self.metadata.len(),
                    Some(record_index),
                    "record-metadata",
                    "Compact metadata cannot use the reserved absent sentinel",
                ));
            }
            self.metadata.push(CompactRecordMetadata {
                datatype,
                identity,
                origin: view.origin.copied(),
                span: view.span,
                extension_start,
                extension_count,
            });
            index
        } else {
            ABSENT_INDEX
        };
        self.records.push(CompactFilmRecord {
            address,
            value,
            metadata_index,
            kind_code,
            is_header: matches!(view.address, crate::film::FilmAddressView::Header(_)),
        });
        if self.retain_path_evidence {
            self.path_evidence.push(path_evidence);
        }
        Ok(())
    }

    fn push_datatype(
        &mut self,
        view: &crate::film::FilmDatatypeView<'_>,
        record_index: usize,
    ) -> Result<u32, FilmError> {
        let index = u32_index(self.datatypes.len(), Some(record_index), "datatype")?;
        let name = self.push_text(view.datatype, Some(record_index), "datatype-name")?;
        self.datatypes.push(CompactDatatype {
            name,
            generic_start: 0,
            generic_count: 0,
            clarifier_start: 0,
            clarifier_count: 0,
        });

        let generic_start = u32_index(self.generics.len(), Some(record_index), "generics")?;
        let generic_count = u32_index(view.generics.len(), Some(record_index), "generics")?;
        let placeholder = CompactGeneric::NumberLiteral(CompactText { start: 0, len: 0 });
        self.generics
            .resize(self.generics.len() + view.generics.len(), placeholder);
        for (offset, generic) in view.generics.iter().enumerate() {
            let compact = match generic {
                crate::film::FilmGenericView::Datatype(nested) => {
                    CompactGeneric::Datatype(self.push_datatype(nested, record_index)?)
                }
                crate::film::FilmGenericView::NumberLiteral(value) => {
                    CompactGeneric::NumberLiteral(self.push_text(
                        value,
                        Some(record_index),
                        "generic-number",
                    )?)
                }
            };
            self.generics[generic_start as usize + offset] = compact;
        }

        let clarifier_start = u32_index(self.clarifiers.len(), Some(record_index), "clarifiers")?;
        for clarifier in &view.clarifiers {
            let value = self.push_text(clarifier.value, Some(record_index), "clarifier-value")?;
            self.clarifiers.push(CompactClarifier {
                kind: clarifier.kind.clone(),
                value,
            });
        }
        let clarifier_count = u32_index(view.clarifiers.len(), Some(record_index), "clarifiers")?;
        self.datatypes[index as usize] = CompactDatatype {
            name,
            generic_start,
            generic_count,
            clarifier_start,
            clarifier_count,
        };
        Ok(index)
    }

    fn finish(self) -> CompactFilmStream {
        CompactFilmStream {
            profile: self.profile,
            profile_explicit: self.profile_explicit,
            projection: self.projection,
            projection_explicit: self.projection_explicit,
            records: self.records,
            datatypes: self.datatypes,
            generics: self.generics,
            clarifiers: self.clarifiers,
            extensions: self.extensions,
            metadata: self.metadata,
            path_evidence: self.path_evidence,
            bytes: self.bytes,
            input_bytes: self.input_bytes,
        }
    }
}

fn candidate_b_to_c(input: &[u8], limits: &FilmLimits) -> Result<Vec<u8>, FilmError> {
    check_limit(
        "max_input_bytes",
        input.len(),
        limits.max_input_bytes,
        0,
        None,
        "stream",
    )?;
    let mut reader = Cursor::new(input);
    reader.expect_preamble(&FILM_CANDIDATE_B_PREAMBLE, "candidate-b-preamble")?;
    let context_end = reader.read_context_end(limits)?;
    let mut output = buffer_with_capacity(input.len(), None, "stream")?;
    output.extend_from_slice(&FILM_CANDIDATE_C_PREAMBLE);
    output.extend_from_slice(&input[FILM_CANDIDATE_B_PREAMBLE.len()..context_end]);

    let mut previous_body = Vec::new();
    let mut previous_header = Vec::new();
    let mut body_initialized = false;
    let mut header_initialized = false;
    let mut record_index = 0_usize;
    while !reader.is_empty() {
        if record_index.is_multiple_of(FILM_CANDIDATE_C_CHECKPOINT_INTERVAL) {
            body_initialized = false;
            header_initialized = false;
        }
        reader.record = Some(record_index);
        let payload_length = reader.read_limited_length(
            "max_record_bytes",
            limits.max_record_bytes,
            "record-length",
            "record",
        )?;
        let payload_offset = reader.absolute_position();
        let payload = reader.read_exact(payload_length, "record")?;
        let mut record = Cursor::with_base(payload, payload_offset, Some(record_index));
        let control = record.read_byte("record-control")?;
        let kind = record.read_byte("kind")?;
        let prefix_value = record.read_uleb("address-prefix")?;
        let suffix = record.read_string_bytes(limits, "address-suffix")?;
        let previous = if control & HEADER_PLANE == 0 {
            &mut previous_body
        } else {
            &mut previous_header
        };
        let plane_initialized = if control & HEADER_PLANE == 0 {
            &mut body_initialized
        } else {
            &mut header_initialized
        };
        let prefix_length = usize::try_from(prefix_value).map_err(|_| {
            error(
                "FILM_INTEGER_OVERFLOW",
                payload_offset,
                Some(record_index),
                "address-prefix",
                "Address prefix length exceeds the host address space",
            )
        })?;
        if prefix_length > previous.len() || !is_char_boundary(previous, prefix_length) {
            return Err(error(
                "FILM_COMPARATOR_INVALID_PREFIX",
                payload_offset,
                Some(record_index),
                "address-prefix",
                "Address prefix exceeds the previous address or splits UTF-8",
            ));
        }
        let address_length = prefix_length.checked_add(suffix.len()).ok_or_else(|| {
            error(
                "FILM_INTEGER_OVERFLOW",
                payload_offset,
                Some(record_index),
                "address",
                "Reconstructed address length overflow",
            )
        })?;
        let mut address = buffer_with_capacity(address_length, Some(record_index), "address")?;
        address.extend_from_slice(&previous[..prefix_length]);
        address.extend_from_slice(suffix);
        std::str::from_utf8(&address).map_err(|invalid| {
            error(
                "FILM_INVALID_UTF8",
                payload_offset.saturating_add(invalid.valid_up_to()),
                Some(record_index),
                "address",
                "Reconstructed address is not UTF-8",
            )
        })?;

        let checkpoint = !*plane_initialized;
        let token = if checkpoint {
            1_u64
        } else {
            prefix_value.checked_mul(2).ok_or_else(|| {
                error(
                    "FILM_INTEGER_OVERFLOW",
                    payload_offset,
                    Some(record_index),
                    "address-token",
                    "Tagged prefix token exceeds u64",
                )
            })?
        };
        let selected_suffix = if checkpoint {
            address.as_slice()
        } else {
            suffix
        };
        let compressed_size = checked_size(
            checked_size(
                checked_size(
                    2,
                    uleb_width(token),
                    payload_offset,
                    Some(record_index),
                    "record",
                )?,
                encoded_string_size(selected_suffix.len(), payload_offset, Some(record_index))?,
                payload_offset,
                Some(record_index),
                "record",
            )?,
            record.remaining().len(),
            payload_offset,
            Some(record_index),
            "record",
        )?;
        check_limit(
            "max_record_bytes",
            compressed_size,
            limits.max_record_bytes,
            payload_offset,
            Some(record_index),
            "record",
        )?;
        let mut compressed = buffer_with_capacity(compressed_size, Some(record_index), "record")?;
        compressed.push(control);
        compressed.push(kind);
        push_uleb(&mut compressed, token);
        push_string_bytes(
            &mut compressed,
            selected_suffix,
            limits,
            Some(record_index),
            "address-suffix",
        )?;
        compressed.extend_from_slice(record.remaining());
        let frame_size = checked_size(
            uleb_width(usize_to_u64(
                compressed.len(),
                payload_offset,
                record_index,
            )?),
            compressed.len(),
            payload_offset,
            Some(record_index),
            "record",
        )?;
        let projected_size = checked_size(
            output.len(),
            frame_size,
            payload_offset,
            Some(record_index),
            "stream",
        )?;
        check_limit(
            "max_input_bytes",
            projected_size,
            limits.max_input_bytes,
            output.len(),
            Some(record_index),
            "stream",
        )?;
        output.try_reserve(frame_size).map_err(|_| {
            error(
                "FILM_INTEGER_OVERFLOW",
                output.len(),
                Some(record_index),
                "stream",
                "Candidate C output cannot grow in the host size domain",
            )
        })?;
        push_uleb(
            &mut output,
            usize_to_u64(compressed.len(), payload_offset, record_index)?,
        );
        output.extend_from_slice(&compressed);
        *previous = address;
        *plane_initialized = true;
        record_index = record_index.saturating_add(1);
    }
    Ok(output)
}

fn read_nonempty_context<'a>(
    reader: &mut Cursor<'a>,
    limits: &FilmLimits,
    component: &'static str,
) -> Result<&'a str, FilmError> {
    let offset = reader.absolute_position();
    let bytes = reader.read_string_bytes(limits, component)?;
    if bytes.is_empty() {
        return Err(error(
            "FILM_INVALID_CONTEXT",
            offset,
            None,
            component,
            "Film context strings must be non-empty",
        ));
    }
    std::str::from_utf8(bytes).map_err(|invalid| {
        error(
            "FILM_INVALID_UTF8",
            offset.saturating_add(invalid.valid_up_to()),
            None,
            component,
            "Film context string is not UTF-8",
        )
    })
}

fn read_context_end_validated(
    reader: &mut Cursor<'_>,
    limits: &FilmLimits,
) -> Result<usize, FilmError> {
    let context = reader.read_byte("stream-context")?;
    if context & !0x03 != 0 {
        return Err(error(
            "FILM_COMPARATOR_NONCANONICAL",
            reader.absolute_position().saturating_sub(1),
            None,
            "stream-context",
            "Reserved context bits must be zero",
        ));
    }
    if context & 0x01 != 0 {
        read_nonempty_context(reader, limits, "profile")?;
    }
    if context & 0x02 != 0 {
        read_nonempty_context(reader, limits, "projection")?;
    }
    Ok(reader.position)
}

fn encode_origin(origin: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(71);
    result.push_str("sha256:");
    for byte in origin {
        result.push(char::from(HEX[usize::from(byte >> 4)]));
        result.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    result
}

fn u32_index(
    value: usize,
    record: Option<usize>,
    component: &'static str,
) -> Result<u32, FilmError> {
    u32::try_from(value).map_err(|_| {
        error(
            "FILM_INTEGER_OVERFLOW",
            value,
            record,
            component,
            "Compact side table exceeds u32 indices",
        )
    })
}

fn common_utf8_prefix_bytes(left: &[u8], right: &[u8]) -> Result<usize, FilmError> {
    let left_text = std::str::from_utf8(left).map_err(|invalid| {
        error(
            "FILM_INVALID_UTF8",
            invalid.valid_up_to(),
            None,
            "previous-address",
            "Previous address is not valid UTF-8",
        )
    })?;
    let right_text = std::str::from_utf8(right).map_err(|invalid| {
        error(
            "FILM_INVALID_UTF8",
            invalid.valid_up_to(),
            None,
            "address",
            "Address is not valid UTF-8",
        )
    })?;
    let mut length = left
        .iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count();
    while !left_text.is_char_boundary(length) || !right_text.is_char_boundary(length) {
        length = length.saturating_sub(1);
    }
    Ok(length)
}

fn is_char_boundary(bytes: &[u8], index: usize) -> bool {
    std::str::from_utf8(bytes).is_ok_and(|value| value.is_char_boundary(index))
}

fn push_string_bytes(
    output: &mut Vec<u8>,
    value: &[u8],
    limits: &FilmLimits,
    record: Option<usize>,
    component: &'static str,
) -> Result<(), FilmError> {
    check_limit(
        "max_field_bytes",
        value.len(),
        limits.max_field_bytes,
        output.len(),
        record,
        component,
    )?;
    let length = u64::try_from(value.len()).map_err(|_| {
        error(
            "FILM_INTEGER_OVERFLOW",
            output.len(),
            record,
            component,
            "String length exceeds u64",
        )
    })?;
    push_uleb(output, length);
    output.extend_from_slice(value);
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

fn uleb_width(mut value: u64) -> usize {
    let mut width = 1_usize;
    while value >= 0x80 {
        value >>= 7;
        width = width.saturating_add(1);
    }
    width
}

fn encoded_string_size(
    length: usize,
    offset: usize,
    record: Option<usize>,
) -> Result<usize, FilmError> {
    let value = u64::try_from(length).map_err(|_| {
        error(
            "FILM_INTEGER_OVERFLOW",
            offset,
            record,
            "length",
            "String length exceeds u64",
        )
    })?;
    checked_size(uleb_width(value), length, offset, record, "string")
}

fn checked_size(
    current: usize,
    added: usize,
    offset: usize,
    record: Option<usize>,
    component: &'static str,
) -> Result<usize, FilmError> {
    current.checked_add(added).ok_or_else(|| {
        error(
            "FILM_INTEGER_OVERFLOW",
            offset,
            record,
            component,
            "Film comparator size exceeds the host address space",
        )
    })
}

fn buffer_with_capacity(
    capacity: usize,
    record: Option<usize>,
    component: &'static str,
) -> Result<Vec<u8>, FilmError> {
    let mut output = Vec::new();
    output.try_reserve_exact(capacity).map_err(|_| {
        error(
            "FILM_INTEGER_OVERFLOW",
            0,
            record,
            component,
            "Film comparator buffer cannot be allocated in the host size domain",
        )
    })?;
    Ok(output)
}

fn usize_to_u64(value: usize, offset: usize, record: usize) -> Result<u64, FilmError> {
    u64::try_from(value).map_err(|_| {
        error(
            "FILM_INTEGER_OVERFLOW",
            offset,
            Some(record),
            "length",
            "Length exceeds u64",
        )
    })
}

fn check_limit(
    counter: &'static str,
    observed: usize,
    limit: usize,
    offset: usize,
    record: Option<usize>,
    component: &'static str,
) -> Result<(), FilmError> {
    if observed > limit {
        Err(error(
            "FILM_LIMIT_EXCEEDED",
            offset,
            record,
            component,
            format!("{counter} observed {observed}, limit {limit}"),
        ))
    } else {
        Ok(())
    }
}

fn error(
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

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
    base_offset: usize,
    record: Option<usize>,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self::with_base(bytes, 0, None)
    }

    fn with_base(bytes: &'a [u8], base_offset: usize, record: Option<usize>) -> Self {
        Self {
            bytes,
            position: 0,
            base_offset,
            record,
        }
    }

    fn absolute_position(&self) -> usize {
        self.base_offset.saturating_add(self.position)
    }

    fn is_empty(&self) -> bool {
        self.position == self.bytes.len()
    }

    fn remaining(&self) -> &'a [u8] {
        self.bytes.get(self.position..).unwrap_or_default()
    }

    fn expect_preamble(
        &mut self,
        expected: &[u8],
        component: &'static str,
    ) -> Result<(), FilmError> {
        if self.bytes.len() < expected.len() {
            return Err(error(
                "FILM_TRUNCATED",
                self.bytes.len(),
                None,
                component,
                "Input ended inside the comparator preamble",
            ));
        }
        if self.bytes.get(..expected.len()) != Some(expected) {
            return Err(error(
                "FILM_INVALID_PREAMBLE",
                0,
                None,
                component,
                "Unexpected comparator preamble",
            ));
        }
        self.position = expected.len();
        Ok(())
    }

    fn read_context_end(&mut self, limits: &FilmLimits) -> Result<usize, FilmError> {
        let control = self.read_byte("stream-context")?;
        if control & !0x03 != 0 {
            return Err(error(
                "FILM_COMPARATOR_NONCANONICAL",
                self.absolute_position().saturating_sub(1),
                None,
                "stream-context",
                "Reserved context bits must be zero",
            ));
        }
        if control & 0x01 != 0 {
            self.skip_string(limits, "profile")?;
        }
        if control & 0x02 != 0 {
            self.skip_string(limits, "projection")?;
        }
        Ok(self.position)
    }

    fn read_byte(&mut self, component: &'static str) -> Result<u8, FilmError> {
        let byte = self.bytes.get(self.position).copied().ok_or_else(|| {
            error(
                "FILM_TRUNCATED",
                self.absolute_position(),
                self.record,
                component,
                "Input ended before a required byte",
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
            error(
                "FILM_INTEGER_OVERFLOW",
                self.absolute_position(),
                self.record,
                component,
                "Byte range overflow",
            )
        })?;
        let result = self.bytes.get(self.position..end).ok_or_else(|| {
            error(
                "FILM_TRUNCATED",
                self.absolute_position(),
                self.record,
                component,
                "Input ended inside a declared field or record",
            )
        })?;
        self.position = end;
        Ok(result)
    }

    fn read_uleb(&mut self, component: &'static str) -> Result<u64, FilmError> {
        let start = self.absolute_position();
        let mut value = 0_u64;
        for index in 0..10_u32 {
            let byte = self.read_byte(component)?;
            let payload = u64::from(byte & 0x7f);
            if index == 9 && payload > 1 {
                return Err(error(
                    "FILM_INTEGER_OVERFLOW",
                    start,
                    self.record,
                    component,
                    "Unsigned LEB128 exceeds u64",
                ));
            }
            value |= payload << (index * 7);
            if byte & 0x80 == 0 {
                let width = usize::try_from(index + 1).unwrap_or(10);
                if width != uleb_width(value) {
                    return Err(error(
                        "FILM_COMPARATOR_NONCANONICAL",
                        start,
                        self.record,
                        component,
                        "Unsigned LEB128 must use its shortest representation",
                    ));
                }
                return Ok(value);
            }
        }
        Err(error(
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
            return Err(error(
                "FILM_LIMIT_EXCEEDED",
                offset,
                self.record,
                limit_component,
                format!("{counter} observed {value}, limit {limit}"),
            ));
        }
        usize::try_from(value).map_err(|_| {
            error(
                "FILM_INTEGER_OVERFLOW",
                offset,
                self.record,
                component,
                "Length exceeds the host address space",
            )
        })
    }

    fn read_string_bytes(
        &mut self,
        limits: &FilmLimits,
        component: &'static str,
    ) -> Result<&'a [u8], FilmError> {
        let offset = self.absolute_position();
        let length = self.read_limited_length(
            "max_field_bytes",
            limits.max_field_bytes,
            component,
            component,
        )?;
        let bytes = self.read_exact(length, component)?;
        std::str::from_utf8(bytes).map_err(|invalid| {
            error(
                "FILM_INVALID_UTF8",
                offset.saturating_add(invalid.valid_up_to()),
                self.record,
                component,
                "String is not valid UTF-8",
            )
        })?;
        Ok(bytes)
    }

    fn skip_string(
        &mut self,
        limits: &FilmLimits,
        component: &'static str,
    ) -> Result<(), FilmError> {
        let length = self.read_limited_length(
            "max_field_bytes",
            limits.max_field_bytes,
            component,
            component,
        )?;
        self.read_exact(length, component)?;
        Ok(())
    }
}
