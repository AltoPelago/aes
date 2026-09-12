use super::{
    AEON_DOCUMENT_PROJECTION, COMPLETE_AES_PROFILE, Diagnostic, EventCandidate,
    PARTIAL_AES_PROFILE, TelexLimits, TelexRecord, TelexSyntaxError, VERSION_LINE,
    ValidationResult, decode_payload_bounded, decode_wire_record, enforce_syntax_limit,
    finalize_event_candidates, has_canonical_field_order, limit_diagnostic,
    prepare_event_candidates_into, valid_field_name,
};
use std::collections::HashSet;

#[cfg(test)]
use super::{ParsedTelex, TELEX_VERSION};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Header,
    Records,
}

#[allow(dead_code)]
#[derive(Debug)]
struct Completion {
    profile: String,
    profile_explicit: bool,
    projection: Option<String>,
    projection_explicit: bool,
    canonical: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct StreamContext {
    profile: String,
    profile_explicit: bool,
    projection: Option<String>,
    projection_explicit: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Progress {
    NeedMoreInput,
    ProvisionalRecords,
    SyntaxComplete,
}

#[allow(dead_code)]
#[derive(Debug)]
struct Update {
    first_record: usize,
    records: Vec<TelexRecord>,
    context: Option<StreamContext>,
    total_input_bytes: usize,
    decoded_payload_bytes: usize,
    canonical_so_far: bool,
    progress: Progress,
    completion: Option<Completion>,
}

/// Internal byte-oriented Telex decoder used to prove chunk-boundary semantics.
///
/// Records are owned and provisional. Whole-stream AES validation remains a
/// separate phase and must succeed before a caller commits them.
#[derive(Debug)]
#[allow(dead_code)]
struct Decoder {
    limits: TelexLimits,
    phase: Phase,
    line: Vec<u8>,
    line_overflow_bytes: Option<usize>,
    line_overflow_ends_cr: bool,
    processed_lines: usize,
    total_input_bytes: usize,
    decoded_payload_bytes: usize,
    saw_input: bool,
    last_was_lf: bool,
    canonical: bool,
    version_seen: bool,
    profile: String,
    profile_explicit: bool,
    projection: Option<String>,
    projection_explicit: bool,
    last_header_rank: Option<usize>,
    separator_width: usize,
    fields: Option<Vec<(String, String)>>,
    datatype_line: Option<usize>,
    datatype_component_line: Option<usize>,
    record_count: usize,
    context_announced: bool,
    completion: bool,
    failure: Option<TelexSyntaxError>,
}

/// Internal semantic sink for provisional record batches.
///
/// Event-local work is performed as batches arrive. Cross-record structure,
/// references, identity uniqueness, and final acceptance are withheld until
/// `finish`, so a downstream transactional consumer cannot mistake a prefix
/// for an accepted AES stream.
pub(super) struct SemanticAccumulator {
    profile: String,
    projection: Option<String>,
    registered_fields: Vec<String>,
    limits: TelexLimits,
    records: Vec<TelexRecord>,
    events: Vec<EventCandidate>,
    diagnostics: Vec<Diagnostic>,
    body_seen: bool,
}

#[allow(dead_code)]
pub(super) struct FinalizedStream {
    pub(super) records: Vec<TelexRecord>,
    pub(super) validation: ValidationResult,
}

impl SemanticAccumulator {
    pub(super) fn new(
        profile: &str,
        projection: Option<&str>,
        registered_fields: &[&str],
        limits: &TelexLimits,
    ) -> Self {
        let mut diagnostics = Vec::new();
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
        Self {
            profile: profile.to_owned(),
            projection: projection.map(str::to_owned),
            registered_fields: registered_fields
                .iter()
                .map(|field| (*field).to_owned())
                .collect(),
            limits: *limits,
            records: Vec::new(),
            events: Vec::new(),
            diagnostics,
            body_seen: false,
        }
    }

    pub(super) fn push_batch(
        &mut self,
        first_record: usize,
        records: Vec<TelexRecord>,
    ) -> Result<(), &'static str> {
        if first_record != self.records.len() {
            return Err("AES record batch ordinal is not contiguous");
        }
        if records.is_empty() {
            return Ok(());
        }
        let start_index = self.records.len();
        self.records.extend(records);
        let registered = self
            .registered_fields
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        prepare_event_candidates_into(
            &self.records,
            start_index,
            &self.profile,
            self.projection.as_deref(),
            &registered,
            &self.limits,
            &mut self.body_seen,
            &mut self.diagnostics,
            &mut self.events,
        );
        Ok(())
    }

    pub(super) fn finish(mut self) -> FinalizedStream {
        if self.records.len() > self.limits.max_events {
            self.diagnostics.insert(
                0,
                limit_diagnostic("max_events", self.records.len(), self.limits.max_events),
            );
        }
        finalize_event_candidates(
            &self.records,
            &self.events,
            &self.profile,
            self.projection.as_deref(),
            &self.limits,
            &mut self.diagnostics,
        );
        let validation = ValidationResult {
            valid: self.diagnostics.is_empty(),
            profile: self.profile,
            diagnostics: self.diagnostics,
        };
        FinalizedStream {
            records: self.records,
            validation,
        }
    }
}

#[allow(dead_code)]
impl Decoder {
    fn new(limits: &TelexLimits) -> Self {
        Self {
            limits: *limits,
            phase: Phase::Header,
            line: Vec::new(),
            line_overflow_bytes: None,
            line_overflow_ends_cr: false,
            processed_lines: 0,
            total_input_bytes: 0,
            decoded_payload_bytes: 0,
            saw_input: false,
            last_was_lf: false,
            canonical: true,
            version_seen: false,
            profile: COMPLETE_AES_PROFILE.to_owned(),
            profile_explicit: false,
            projection: None,
            projection_explicit: false,
            last_header_rank: None,
            separator_width: 0,
            fields: None,
            datatype_line: None,
            datatype_component_line: None,
            record_count: 0,
            context_announced: false,
            completion: false,
            failure: None,
        }
    }

    fn push(&mut self, input: &[u8], final_chunk: bool) -> Result<Update, TelexSyntaxError> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if self.completion {
            return Err(TelexSyntaxError::new(
                "TELEX_DECODER_CLOSED",
                "The incremental Telex decoder is already complete",
                None,
            ));
        }

        let result = self.push_inner(input, final_chunk);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }

    fn push_inner(&mut self, input: &[u8], final_chunk: bool) -> Result<Update, TelexSyntaxError> {
        let first_record = self.record_count;
        let mut records = Vec::new();
        let observed_input_bytes = self.total_input_bytes.saturating_add(input.len());
        enforce_syntax_limit(
            "max_input_bytes",
            observed_input_bytes,
            self.limits.max_input_bytes,
            None,
        )?;
        self.total_input_bytes = observed_input_bytes;
        if !input.is_empty() {
            self.saw_input = true;
            self.last_was_lf = input.last() == Some(&b'\n');
        }
        self.process_bytes(input, &mut records)?;

        if !final_chunk {
            let progress = if records.is_empty() {
                Progress::NeedMoreInput
            } else {
                Progress::ProvisionalRecords
            };
            return Ok(self.update(first_record, records, progress, None));
        }

        if let Some(raw_length) = self.line_overflow_bytes.take() {
            let observed = raw_length - usize::from(self.line_overflow_ends_cr);
            return Err(TelexSyntaxError::limit(
                "max_line_bytes",
                observed,
                self.limits.max_line_bytes,
                Some(self.processed_lines + 1),
            ));
        }
        if !self.line.is_empty() || !self.saw_input {
            let line = std::mem::take(&mut self.line);
            self.process_line(&line, false, &mut records)?;
        }
        self.finish(&mut records)?;
        self.completion = true;
        let completion = Completion {
            profile: self.profile.clone(),
            profile_explicit: self.profile_explicit,
            projection: self.projection.clone(),
            projection_explicit: self.projection_explicit,
            canonical: self.canonical && self.last_was_lf,
        };
        Ok(self.update(
            first_record,
            records,
            Progress::SyntaxComplete,
            Some(completion),
        ))
    }

    fn process_bytes(
        &mut self,
        mut input: &[u8],
        records: &mut Vec<TelexRecord>,
    ) -> Result<(), TelexSyntaxError> {
        while !input.is_empty() {
            if let Some(raw_length) = &mut self.line_overflow_bytes {
                if let Some(newline) = input.iter().position(|byte| *byte == b'\n') {
                    *raw_length = raw_length.saturating_add(newline);
                    if newline > 0 {
                        self.line_overflow_ends_cr = input[newline - 1] == b'\r';
                    }
                    let observed = *raw_length - usize::from(self.line_overflow_ends_cr);
                    return Err(TelexSyntaxError::limit(
                        "max_line_bytes",
                        observed,
                        self.limits.max_line_bytes,
                        Some(self.processed_lines + 1),
                    ));
                }
                *raw_length = raw_length.saturating_add(input.len());
                self.line_overflow_ends_cr = input.last() == Some(&b'\r');
                return Ok(());
            }

            let newline = input.iter().position(|byte| *byte == b'\n');
            if self.line.is_empty() {
                if let Some(newline) = newline {
                    let bytes = &input[..newline];
                    self.check_line_length(bytes)?;
                    self.process_line(bytes, true, records)?;
                    input = &input[newline + 1..];
                    continue;
                }
                self.retain_partial_line(input);
                return Ok(());
            }

            if let Some(newline) = newline {
                let raw_length = self.line.len().saturating_add(newline);
                let ends_cr = if newline == 0 {
                    self.line.last() == Some(&b'\r')
                } else {
                    input[newline - 1] == b'\r'
                };
                let observed = raw_length - usize::from(ends_cr);
                if observed > self.limits.max_line_bytes {
                    return Err(TelexSyntaxError::limit(
                        "max_line_bytes",
                        observed,
                        self.limits.max_line_bytes,
                        Some(self.processed_lines + 1),
                    ));
                }
                let mut line = std::mem::take(&mut self.line);
                line.extend_from_slice(&input[..newline]);
                self.process_line(&line, true, records)?;
                input = &input[newline + 1..];
                continue;
            }

            let raw_length = self.line.len().saturating_add(input.len());
            let ends_cr = input.last() == Some(&b'\r');
            let observed = raw_length - usize::from(ends_cr);
            if observed > self.limits.max_line_bytes {
                self.line_overflow_bytes = Some(raw_length);
                self.line_overflow_ends_cr = ends_cr;
            } else {
                self.line.extend_from_slice(input);
            }
            return Ok(());
        }
        Ok(())
    }

    fn retain_partial_line(&mut self, input: &[u8]) {
        let ends_cr = input.last() == Some(&b'\r');
        let observed = input.len() - usize::from(ends_cr);
        if observed > self.limits.max_line_bytes {
            self.line_overflow_bytes = Some(input.len());
            self.line_overflow_ends_cr = ends_cr;
        } else {
            self.line.extend_from_slice(input);
        }
    }

    fn check_line_length(&self, bytes: &[u8]) -> Result<(), TelexSyntaxError> {
        let observed = bytes.len() - usize::from(bytes.last() == Some(&b'\r'));
        enforce_syntax_limit(
            "max_line_bytes",
            observed,
            self.limits.max_line_bytes,
            Some(self.processed_lines + 1),
        )
    }

    fn update(
        &mut self,
        first_record: usize,
        records: Vec<TelexRecord>,
        progress: Progress,
        completion: Option<Completion>,
    ) -> Update {
        let context_fixed = self.phase == Phase::Records || completion.is_some();
        let context = if context_fixed && !self.context_announced {
            self.context_announced = true;
            Some(StreamContext {
                profile: self.profile.clone(),
                profile_explicit: self.profile_explicit,
                projection: self.projection.clone(),
                projection_explicit: self.projection_explicit,
            })
        } else {
            None
        };
        Update {
            first_record,
            records,
            context,
            total_input_bytes: self.total_input_bytes,
            decoded_payload_bytes: self.decoded_payload_bytes,
            canonical_so_far: self.canonical,
            progress,
            completion,
        }
    }

    fn process_line(
        &mut self,
        bytes: &[u8],
        terminated: bool,
        records: &mut Vec<TelexRecord>,
    ) -> Result<(), TelexSyntaxError> {
        let line_number = self.processed_lines + 1;
        let has_suffix_cr = bytes.last() == Some(&b'\r');
        let physical_length = bytes.len() - usize::from(has_suffix_cr);
        enforce_syntax_limit(
            "max_line_bytes",
            physical_length,
            self.limits.max_line_bytes,
            Some(line_number),
        )?;
        let bytes = if terminated && has_suffix_cr {
            self.canonical = false;
            &bytes[..bytes.len() - 1]
        } else {
            bytes
        };
        if bytes.contains(&b'\r') {
            return Err(TelexSyntaxError::new(
                "TELEX_BARE_CR",
                "Bare carriage returns are not allowed",
                None,
            ));
        }
        let line = std::str::from_utf8(bytes).map_err(|_| {
            TelexSyntaxError::new(
                "TELEX_INVALID_UTF8",
                "Telex input must be valid UTF-8",
                Some(line_number),
            )
        })?;
        if line_number == 1 && line.starts_with('\u{feff}') {
            return Err(TelexSyntaxError::new(
                "TELEX_BOM",
                "UTF-8 byte-order marks are not allowed",
                Some(1),
            ));
        }
        self.processed_lines += 1;

        match self.phase {
            Phase::Header => self.process_header_line(line, line_number),
            Phase::Records => self.process_record_line(line, line_number, records),
        }
    }

    fn process_header_line(
        &mut self,
        line: &str,
        line_number: usize,
    ) -> Result<(), TelexSyntaxError> {
        if !self.version_seen {
            if line != VERSION_LINE {
                return Err(TelexSyntaxError::new(
                    "TELEX_INVALID_PREAMBLE",
                    format!("Expected {VERSION_LINE}"),
                    Some(1),
                ));
            }
            self.version_seen = true;
            return Ok(());
        }
        if line.is_empty() {
            self.phase = Phase::Records;
            self.separator_width = 1;
            return Ok(());
        }

        let Some((field, payload)) = line.split_once('=') else {
            return self.missing_header_separator(line_number);
        };
        let rank = match field {
            "profile" => 0,
            "projection" => 1,
            _ => return self.missing_header_separator(line_number),
        };
        if self
            .last_header_rank
            .is_some_and(|previous| rank < previous)
        {
            self.canonical = false;
        }
        self.last_header_rank = Some(rank);
        let decoded = decode_payload_bounded(
            payload,
            line_number,
            &self.limits,
            &mut self.decoded_payload_bytes,
        )?;
        self.canonical &= decoded.canonical;
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
            if self.profile_explicit {
                return Err(TelexSyntaxError::new(
                    "TELEX_DUPLICATE_STREAM_FIELD",
                    "Duplicate stream field: profile",
                    Some(line_number),
                ));
            }
            self.profile = decoded.value;
            self.profile_explicit = true;
        } else {
            if self.projection_explicit {
                return Err(TelexSyntaxError::new(
                    "TELEX_DUPLICATE_STREAM_FIELD",
                    "Duplicate stream field: projection",
                    Some(line_number),
                ));
            }
            self.projection = Some(decoded.value);
            self.projection_explicit = true;
        }
        Ok(())
    }

    fn missing_header_separator(&self, line: usize) -> Result<(), TelexSyntaxError> {
        Err(TelexSyntaxError::new(
            "TELEX_MISSING_HEADER_SEPARATOR",
            "Expected a blank line after the stream header",
            Some(line),
        ))
    }

    fn process_record_line(
        &mut self,
        line: &str,
        line_number: usize,
        records: &mut Vec<TelexRecord>,
    ) -> Result<(), TelexSyntaxError> {
        if line.is_empty() {
            self.separator_width += 1;
            if self.fields.is_some() {
                self.close_record(line_number, records)?;
            }
            return Ok(());
        }
        if self.separator_width > 1 {
            self.canonical = false;
        }
        self.separator_width = 0;

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
        let fields = self.fields.get_or_insert_with(Vec::new);
        if fields.iter().any(|(existing, _)| existing == field) {
            return Err(TelexSyntaxError::new(
                "TELEX_DUPLICATE_FIELD",
                format!("Duplicate field: {field}"),
                Some(line_number),
            ));
        }
        enforce_syntax_limit(
            "max_fields_per_event",
            fields.len() + 1,
            self.limits.max_fields_per_event,
            Some(line_number),
        )?;
        let decoded = decode_payload_bounded(
            &line[delimiter + 1..],
            line_number,
            &self.limits,
            &mut self.decoded_payload_bytes,
        )?;
        self.canonical &= decoded.canonical;
        if field == "datatype" {
            self.datatype_line = Some(line_number);
        } else if (field == "generics" || field == "clarifiers")
            && self.datatype_component_line.is_none()
        {
            self.datatype_component_line = Some(line_number);
        }
        fields.push((field.to_owned(), decoded.value));
        Ok(())
    }

    fn close_record(
        &mut self,
        line_number: usize,
        records: &mut Vec<TelexRecord>,
    ) -> Result<(), TelexSyntaxError> {
        enforce_syntax_limit(
            "max_events",
            self.record_count + 1,
            self.limits.max_events,
            Some(line_number),
        )?;
        let fields = self.fields.take().expect("record fields checked");
        let (record, descriptor_canonical) = decode_wire_record(
            fields,
            self.datatype_line.take(),
            self.datatype_component_line.take(),
            &self.limits,
        )?;
        self.canonical &= descriptor_canonical && has_canonical_field_order(&record);
        self.record_count += 1;
        records.push(record);
        Ok(())
    }

    fn finish(&mut self, records: &mut Vec<TelexRecord>) -> Result<(), TelexSyntaxError> {
        if !self.version_seen {
            return Err(TelexSyntaxError::new(
                "TELEX_INVALID_PREAMBLE",
                format!("Expected {VERSION_LINE}"),
                Some(1),
            ));
        }
        if self.phase == Phase::Records {
            if self.fields.is_some() {
                self.close_record(self.processed_lines, records)?;
            }
            if self.separator_width > 0 {
                self.canonical = false;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        encode_telex_with_projection, parse_telex, parse_telex_with_limits, validate_telex,
        validate_telex_with_limits,
    };
    use serde_json::Value;
    use std::fs;
    use std::path::Path;
    use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
    use std::thread;
    use std::time::{Duration, Instant};

    #[cfg(feature = "pipeline-dhat")]
    use std::{env, hint::black_box, process::Command};

    #[cfg(feature = "pipeline-dhat")]
    #[global_allocator]
    static ALLOCATOR: dhat::Alloc = dhat::Alloc;

    fn parse_in_chunks(
        input: &[u8],
        split_points: &[usize],
    ) -> Result<ParsedTelex, TelexSyntaxError> {
        parse_in_chunks_with_limits(input, split_points, &TelexLimits::default())
    }

    fn parse_in_chunks_with_limits(
        input: &[u8],
        split_points: &[usize],
        limits: &TelexLimits,
    ) -> Result<ParsedTelex, TelexSyntaxError> {
        let mut decoder = Decoder::new(limits);
        let mut records = Vec::new();
        let mut start = 0;
        for &end in split_points {
            let update = decoder.push(&input[start..end], false)?;
            records.extend(update.records);
            start = end;
        }
        let update = decoder.push(&input[start..], true)?;
        records.extend(update.records);
        let completion = update.completion.expect("final update completes");
        Ok(ParsedTelex {
            version: TELEX_VERSION.to_owned(),
            profile: completion.profile,
            profile_explicit: completion.profile_explicit,
            projection: completion.projection,
            projection_explicit: completion.projection_explicit,
            records,
            canonical: completion.canonical,
        })
    }

    fn validate_one_byte_pipeline(
        input: &str,
        registered_fields: &[&str],
    ) -> Result<ValidationResult, TelexSyntaxError> {
        let limits = TelexLimits::default();
        validate_one_byte_pipeline_with_limits(input, registered_fields, &limits)
    }

    fn validate_one_byte_pipeline_with_limits(
        input: &str,
        registered_fields: &[&str],
        limits: &TelexLimits,
    ) -> Result<ValidationResult, TelexSyntaxError> {
        let mut decoder = Decoder::new(limits);
        let mut accumulator = None;
        for (index, byte) in input.as_bytes().iter().enumerate() {
            let update = decoder.push(std::slice::from_ref(byte), index + 1 == input.len())?;
            if !update.records.is_empty() {
                let sink = accumulator.get_or_insert_with(|| {
                    SemanticAccumulator::new(
                        &decoder.profile,
                        decoder.projection.as_deref(),
                        registered_fields,
                        limits,
                    )
                });
                sink.push_batch(update.first_record, update.records)
                    .expect("decoder record ordinals are contiguous");
            }
        }
        if input.is_empty() {
            decoder.push(b"", true)?;
        }
        let accumulator = accumulator.unwrap_or_else(|| {
            SemanticAccumulator::new(
                &decoder.profile,
                decoder.projection.as_deref(),
                registered_fields,
                limits,
            )
        });
        Ok(accumulator.finish().validation)
    }

    #[test]
    fn every_single_split_matches_one_shot() {
        let input = concat!(
            "telex.aes=1\n",
            "profile=aes.partial.v1\n",
            "projection=aeon.document.v1\n",
            "\n",
            "path=$.greeting\n",
            "kind=StringLiteral\n",
            "datatype=list<string>\n",
            "value=héllo\\nworld\n",
            "\n",
            "path=$.answer\n",
            "kind=NumberLiteral\n",
            "value=42\n",
        );
        let expected = parse_telex(input).expect("one-shot parse");
        for split in 0..=input.len() {
            assert_eq!(
                parse_in_chunks(input.as_bytes(), &[split]).expect("incremental parse"),
                expected,
                "split at byte {split}",
            );
        }
    }

    #[test]
    fn one_byte_chunks_match_one_shot() {
        let input = "telex.aes=1\r\n\r\npath=$.é\r\nkind=StringLiteral\r\nvalue=x\\ty\r\n";
        let expected = parse_telex(input).expect("one-shot parse");
        let splits: Vec<_> = (0..input.len()).collect();
        assert_eq!(
            parse_in_chunks(input.as_bytes(), &splits).expect("incremental parse"),
            expected,
        );
    }

    #[test]
    fn records_are_emitted_only_at_complete_boundaries() {
        let input = "telex.aes=1\n\npath=$.a\nkind=StringLiteral\nvalue=one\n\npath=$.b\nkind=StringLiteral\nvalue=two\n";
        let boundary = input.find("\n\npath=$.b").expect("record boundary") + 2;
        let mut decoder = Decoder::new(&TelexLimits::default());
        let first = decoder
            .push(&input.as_bytes()[..boundary], false)
            .expect("first push");
        assert_eq!(first.records.len(), 1);
        assert_eq!(first.first_record, 0);
        assert_eq!(first.progress, Progress::ProvisionalRecords);
        assert_eq!(
            first
                .context
                .as_ref()
                .map(|context| context.profile.as_str()),
            Some(COMPLETE_AES_PROFILE),
        );
        assert_eq!(first.total_input_bytes, boundary);
        assert!(first.completion.is_none());
        let final_update = decoder
            .push(&input.as_bytes()[boundary..], true)
            .expect("final push");
        assert_eq!(final_update.records.len(), 1);
        assert_eq!(final_update.first_record, 1);
        assert_eq!(final_update.progress, Progress::SyntaxComplete);
        assert!(final_update.context.is_none());
        assert_eq!(final_update.total_input_bytes, input.len());
        assert!(final_update.canonical_so_far);
        assert!(final_update.completion.is_some());
    }

    #[test]
    fn invalid_utf8_and_failures_are_sticky() {
        let mut decoder = Decoder::new(&TelexLimits::default());
        let invalid = decoder
            .push(b"telex.aes=1\n\nvalue=\xFF\n", true)
            .expect_err("invalid UTF-8");
        assert_eq!(invalid.code, "TELEX_INVALID_UTF8");
        assert_eq!(invalid.line, Some(3));
        assert_eq!(
            decoder.push(b"", true).expect_err("sticky failure"),
            invalid
        );
    }

    #[test]
    fn canonical_encoder_output_matches_all_split_positions() {
        let parsed = parse_telex("telex.aes=1\n\npath=$.a\nkind=StringLiteral\nvalue=é\\n\n")
            .expect("fixture parse");
        let encoded = encode_telex_with_projection(
            &parsed.records,
            Some(&parsed.profile),
            parsed.projection.as_deref(),
        )
        .expect("encode");
        let expected = parse_telex(&encoded).expect("one-shot encoded parse");
        for split in 0..=encoded.len() {
            assert_eq!(
                parse_in_chunks(encoded.as_bytes(), &[split]).expect("incremental encoded parse"),
                expected,
            );
        }
    }

    #[test]
    fn syntax_failures_match_at_every_split_position() {
        let fixtures = [
            "wrong\n",
            "\u{feff}telex.aes=1\n",
            "telex.aes=1\rpath=$.a\n",
            "telex.aes=1\npath=$.a\n",
            "telex.aes=1\n\nmissing-delimiter\n",
            "telex.aes=1\n\n=value\n",
            "telex.aes=1\n\npath=$.a\npath=$.b\n",
            "telex.aes=1\n\npath=$.a\\q\n",
            "telex.aes=1\n\npath=$.a\ndatatype=list<string\n",
        ];
        for input in fixtures {
            let expected = parse_telex(input).expect_err("one-shot fixture must fail");
            for split in 0..=input.len() {
                let actual = parse_in_chunks(input.as_bytes(), &[split])
                    .expect_err("incremental fixture must fail");
                assert_eq!(actual, expected, "fixture {input:?}, split at byte {split}");
            }
        }
    }

    #[test]
    fn physical_limit_failures_match_at_every_split_position() {
        let input = "telex.aes=1\n\npath=$.abcdef\nkind=StringLiteral\nvalue=x\n";
        let limits = TelexLimits {
            max_line_bytes: 14,
            ..TelexLimits::default()
        };
        let expected = parse_telex_with_limits(input, &limits).expect_err("line limit");
        for split in 0..=input.len() {
            let actual = parse_in_chunks_with_limits(input.as_bytes(), &[split], &limits)
                .expect_err("incremental line limit");
            assert_eq!(actual, expected, "split at byte {split}");
        }

        let limits = TelexLimits {
            max_input_bytes: input.len() - 1,
            ..TelexLimits::default()
        };
        let expected = parse_telex_with_limits(input, &limits).expect_err("input limit");
        for split in 0..=input.len() {
            let actual = parse_in_chunks_with_limits(input.as_bytes(), &[split], &limits)
                .expect_err("incremental input limit");
            assert_same_input_limit_error(&actual, &expected, input.len(), split);
        }
    }

    #[test]
    fn every_local_telex_cts_input_matches_at_all_single_splits_and_bytewise() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/telex/v1");
        let manifest = read_json(&root.join("telex-cts.v1.json"));
        for suite in manifest["suites"]
            .as_array()
            .expect("CTS suites must be an array")
        {
            let suite = read_json(
                &root.join(
                    suite["file"]
                        .as_str()
                        .expect("CTS suite file must be a string"),
                ),
            );
            for vector in suite["tests"]
                .as_array()
                .expect("CTS tests must be an array")
            {
                let id = vector["id"].as_str().expect("CTS id must be a string");
                let input = vector["input"]["telex"]
                    .as_str()
                    .expect("CTS Telex input must be a string");
                let limits = cts_limits(vector);
                let expected = parse_telex_with_limits(input, &limits);
                for split in 0..=input.len() {
                    let actual = parse_in_chunks_with_limits(input.as_bytes(), &[split], &limits);
                    assert_parse_equivalent(&actual, &expected, id, Some(split));
                }
                let byte_splits = (0..input.len()).collect::<Vec<_>>();
                let actual = parse_in_chunks_with_limits(input.as_bytes(), &byte_splits, &limits);
                assert_parse_equivalent(&actual, &expected, id, None);
                if expected.is_ok() {
                    let registered = vector["input"]["registered_fields"]
                        .as_array()
                        .map(|fields| {
                            fields
                                .iter()
                                .map(|field| {
                                    field.as_str().expect("registered field must be a string")
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    assert_eq!(
                        validate_one_byte_pipeline_with_limits(input, &registered, &limits)
                            .expect("CTS syntax already accepted"),
                        validate_telex_with_limits(input, &registered, &limits)
                            .expect("CTS syntax already accepted"),
                        "{id}: incremental semantic validation",
                    );
                }
            }
        }
    }

    #[test]
    fn incremental_semantic_accumulator_matches_one_shot_validation() {
        let fixtures = [
            "telex.aes=1\n",
            concat!(
                "telex.aes=1\n\n",
                "path=$.a\nkind=ObjectNode\n\n",
                "path=$.a.b\nkind=StringLiteral\nvalue=x\n",
            ),
            concat!(
                "telex.aes=1\nprofile=aes.partial.v1\n\n",
                "path=$.missing.child\nkind=StringLiteral\nvalue=x\n",
            ),
            concat!(
                "telex.aes=1\n\n",
                "path=$.a\nkind=StringLiteral\nvalue=x\n\n",
                "path=$.a\nkind=StringLiteral\nvalue=y\n",
            ),
            concat!(
                "telex.aes=1\nprojection=aeon.document.v1\n\n",
                "path=$.a\nkind=StringLiteral\nvalue=x\n\n",
                "header=$.[\"aeon:mode\"]\nkind=StringLiteral\nvalue=strict\n",
            ),
        ];
        for input in fixtures {
            assert_eq!(
                validate_one_byte_pipeline(input, &[]).expect("incremental syntax"),
                validate_telex(input, &[]).expect("one-shot syntax"),
                "fixture {input:?}",
            );
        }
    }

    #[test]
    fn semantic_accumulator_rejects_noncontiguous_batches() {
        let limits = TelexLimits::default();
        let mut accumulator = SemanticAccumulator::new(COMPLETE_AES_PROFILE, None, &[], &limits);
        assert_eq!(
            accumulator.push_batch(1, Vec::new()),
            Err("AES record batch ordinal is not contiguous"),
        );
    }

    #[test]
    #[ignore = "explicit performance experiment"]
    fn benchmark_capacity_one_within_document_pipeline() {
        const EVENT_COUNT: usize = 100_000;
        const WARMUPS: usize = 3;
        let iterations = std::env::var("AES_PIPELINE_BENCH_ITERATIONS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(15);
        assert!(iterations > 0, "benchmark iterations must be positive");

        let input = build_benchmark_telex(EVENT_COUNT);
        let limits = TelexLimits {
            max_list_items: EVENT_COUNT,
            ..TelexLimits::default()
        };
        let one_shot = measure(WARMUPS, iterations, || {
            let validation =
                validate_telex_with_limits(&input, &[], &limits).expect("generated Telex syntax");
            assert!(validation.valid, "generated Telex semantics");
        });
        println!("# benchmark=telex-within-document-pipeline-v1");
        println!("# events={EVENT_COUNT}");
        println!("# warmups={WARMUPS}");
        println!("# iterations={iterations}");
        println!("# handoff=std-sync-channel;capacity=1;owned-provisional-record-batches");
        println!(
            "topology,chunk_bytes,median_ms,p95_ms,events_per_second,speedup,batches,max_batch_records,max_batch_decoded_payload_bytes,queue_full_handoffs"
        );
        emit_benchmark(
            "T0-one-shot",
            0,
            one_shot,
            one_shot,
            EVENT_COUNT,
            PipelineStats::default(),
        );
        for chunk_bytes in [16 * 1024, 64 * 1024, 256 * 1024, 1024 * 1024] {
            let sequential = measure(WARMUPS, iterations, || {
                assert_eq!(
                    run_incremental_sequential(&input, chunk_bytes, &limits),
                    EVENT_COUNT,
                );
            });
            let mut pipeline_stats = PipelineStats::default();
            let threaded = measure(WARMUPS, iterations, || {
                let (event_count, current_stats) =
                    run_incremental_threaded(&input, chunk_bytes, &limits);
                assert_eq!(event_count, EVENT_COUNT);
                pipeline_stats = current_stats;
            });
            emit_benchmark(
                "T0-incremental",
                chunk_bytes,
                sequential,
                one_shot,
                EVENT_COUNT,
                PipelineStats::default(),
            );
            emit_benchmark(
                "T1-capacity-1",
                chunk_bytes,
                threaded,
                one_shot,
                EVENT_COUNT,
                pipeline_stats,
            );
        }
        let mut tile_stats = PipelineStats::default();
        let tile = measure(WARMUPS, iterations, || {
            let (event_count, current_stats) = run_two_lane_incremental(&input, 64 * 1024, &limits);
            assert_eq!(event_count, EVENT_COUNT * 2);
            tile_stats = current_stats;
        });
        emit_benchmark(
            "T2-2x2-capacity-1",
            64 * 1024,
            tile,
            one_shot,
            EVENT_COUNT * 2,
            tile_stats,
        );
    }

    #[test]
    #[ignore = "explicit allocation experiment"]
    #[cfg(feature = "pipeline-dhat")]
    fn profile_capacity_one_within_document_pipeline() {
        const OPERATION_ENV: &str = "AES_PIPELINE_PROFILE_OPERATION";
        const EVENT_COUNT: usize = 100_000;
        const CHUNK_BYTES: usize = 64 * 1024;

        let Ok(operation) = env::var(OPERATION_ENV) else {
            println!("# profiler=dhat-0.3.3");
            println!("# benchmark=telex-within-document-pipeline-v1");
            println!("# events={EVENT_COUNT}");
            println!("# chunk-bytes={CHUNK_BYTES}");
            println!(
                "operation,total_allocations,total_bytes,peak_live_allocations,peak_live_bytes,end_live_allocations,end_live_bytes"
            );
            let executable = env::current_exe().expect("test executable path");
            for operation in ["one-shot", "t1-capacity-1"] {
                let status = Command::new(&executable)
                    .args([
                        "telex_incremental::tests::profile_capacity_one_within_document_pipeline",
                        "--exact",
                        "--ignored",
                        "--nocapture",
                    ])
                    .env(OPERATION_ENV, operation)
                    .status()
                    .expect("allocation profiler child must launch");
                assert!(status.success(), "allocation profiler failed: {operation}");
            }
            return;
        };

        assert!(["one-shot", "t1-capacity-1"].contains(&operation.as_str()));
        let input = build_benchmark_telex(EVENT_COUNT);
        let limits = TelexLimits {
            max_list_items: EVENT_COUNT,
            ..TelexLimits::default()
        };
        run_profile_operation(&operation, &input, CHUNK_BYTES, &limits);
        let profiler = dhat::Profiler::builder().testing().build();
        run_profile_operation(&operation, &input, CHUNK_BYTES, &limits);
        let stats = dhat::HeapStats::get();
        drop(profiler);
        println!(
            "{operation},{},{},{},{},{},{}",
            stats.total_blocks,
            stats.total_bytes,
            stats.max_blocks,
            stats.max_bytes,
            stats.curr_blocks,
            stats.curr_bytes,
        );
    }

    #[derive(Clone, Copy)]
    struct BenchmarkTiming {
        median: Duration,
        p95: Duration,
    }

    #[derive(Clone, Copy, Default)]
    struct PipelineStats {
        batches: usize,
        max_batch_records: usize,
        max_batch_decoded_payload_bytes: usize,
        queue_full_handoffs: usize,
    }

    enum PipelineMessage {
        Context {
            profile: String,
            projection: Option<String>,
        },
        Records {
            first_record: usize,
            records: Vec<TelexRecord>,
        },
        Complete,
    }

    fn run_incremental_sequential(input: &str, chunk_bytes: usize, limits: &TelexLimits) -> usize {
        let mut decoder = Decoder::new(limits);
        let mut accumulator = None;
        for (index, chunk) in input.as_bytes().chunks(chunk_bytes).enumerate() {
            let final_chunk = (index + 1) * chunk_bytes >= input.len();
            let update = decoder
                .push(chunk, final_chunk)
                .expect("generated Telex must decode");
            if !update.records.is_empty() {
                let sink = accumulator.get_or_insert_with(|| {
                    SemanticAccumulator::new(
                        &decoder.profile,
                        decoder.projection.as_deref(),
                        &[],
                        limits,
                    )
                });
                sink.push_batch(update.first_record, update.records)
                    .expect("decoder record ordinals are contiguous");
            }
        }
        finish_benchmark_accumulator(accumulator, &decoder, limits)
    }

    fn run_incremental_threaded(
        input: &str,
        chunk_bytes: usize,
        limits: &TelexLimits,
    ) -> (usize, PipelineStats) {
        let (sender, receiver) = sync_channel(1);
        thread::scope(|scope| {
            let producer = scope
                .spawn(move || produce_incremental_batches(input, chunk_bytes, limits, sender));
            let consumer = scope.spawn(move || consume_incremental_batches(receiver, limits));
            let stats = producer.join().expect("physical decoder must finish");
            (
                consumer.join().expect("semantic accumulator must finish"),
                stats,
            )
        })
    }

    fn run_two_lane_incremental(
        input: &str,
        chunk_bytes: usize,
        limits: &TelexLimits,
    ) -> (usize, PipelineStats) {
        thread::scope(|scope| {
            let lane_a = scope.spawn(move || run_incremental_threaded(input, chunk_bytes, limits));
            let lane_b = scope.spawn(move || run_incremental_threaded(input, chunk_bytes, limits));
            let (events_a, stats_a) = lane_a.join().expect("T2 lane A must finish");
            let (events_b, stats_b) = lane_b.join().expect("T2 lane B must finish");
            (
                events_a + events_b,
                PipelineStats {
                    batches: stats_a.batches + stats_b.batches,
                    max_batch_records: stats_a.max_batch_records.max(stats_b.max_batch_records),
                    max_batch_decoded_payload_bytes: stats_a
                        .max_batch_decoded_payload_bytes
                        .max(stats_b.max_batch_decoded_payload_bytes),
                    queue_full_handoffs: stats_a.queue_full_handoffs + stats_b.queue_full_handoffs,
                },
            )
        })
    }

    fn produce_incremental_batches(
        input: &str,
        chunk_bytes: usize,
        limits: &TelexLimits,
        sender: SyncSender<PipelineMessage>,
    ) -> PipelineStats {
        let mut decoder = Decoder::new(limits);
        let mut stats = PipelineStats::default();
        let mut previous_payload_bytes = 0;
        for (index, chunk) in input.as_bytes().chunks(chunk_bytes).enumerate() {
            let final_chunk = (index + 1) * chunk_bytes >= input.len();
            let update = decoder
                .push(chunk, final_chunk)
                .expect("generated Telex must decode");
            if let Some(context) = update.context {
                sender
                    .send(PipelineMessage::Context {
                        profile: context.profile,
                        projection: context.projection,
                    })
                    .expect("semantic consumer must remain available");
            }
            if !update.records.is_empty() {
                stats.batches += 1;
                stats.max_batch_records = stats.max_batch_records.max(update.records.len());
                stats.max_batch_decoded_payload_bytes = stats
                    .max_batch_decoded_payload_bytes
                    .max(update.decoded_payload_bytes - previous_payload_bytes);
                previous_payload_bytes = update.decoded_payload_bytes;
                let message = PipelineMessage::Records {
                    first_record: update.first_record,
                    records: update.records,
                };
                match sender.try_send(message) {
                    Ok(()) => {}
                    Err(TrySendError::Full(message)) => {
                        stats.queue_full_handoffs += 1;
                        sender
                            .send(message)
                            .expect("semantic consumer must remain available");
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        panic!("semantic consumer must remain available");
                    }
                }
            }
        }
        sender
            .send(PipelineMessage::Complete)
            .expect("semantic consumer must remain available");
        stats
    }

    fn consume_incremental_batches(
        receiver: Receiver<PipelineMessage>,
        limits: &TelexLimits,
    ) -> usize {
        let mut accumulator = None;
        for message in receiver {
            match message {
                PipelineMessage::Context {
                    profile,
                    projection,
                } => {
                    assert!(accumulator.is_none(), "stream context is sent once");
                    accumulator = Some(SemanticAccumulator::new(
                        &profile,
                        projection.as_deref(),
                        &[],
                        limits,
                    ));
                }
                PipelineMessage::Records {
                    first_record,
                    records,
                } => accumulator
                    .as_mut()
                    .expect("context precedes records")
                    .push_batch(first_record, records)
                    .expect("decoder record ordinals are contiguous"),
                PipelineMessage::Complete => break,
            }
        }
        let finalized = accumulator.expect("pipeline context").finish();
        assert!(finalized.validation.valid, "generated Telex semantics");
        finalized.records.len()
    }

    fn finish_benchmark_accumulator(
        accumulator: Option<SemanticAccumulator>,
        decoder: &Decoder,
        limits: &TelexLimits,
    ) -> usize {
        let accumulator = accumulator.unwrap_or_else(|| {
            SemanticAccumulator::new(&decoder.profile, decoder.projection.as_deref(), &[], limits)
        });
        let finalized = accumulator.finish();
        assert!(finalized.validation.valid, "generated Telex semantics");
        finalized.records.len()
    }

    fn measure(warmups: usize, iterations: usize, mut operation: impl FnMut()) -> BenchmarkTiming {
        for _ in 0..warmups {
            operation();
        }
        let mut samples = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            operation();
            samples.push(start.elapsed());
        }
        samples.sort_unstable();
        BenchmarkTiming {
            median: samples[samples.len() / 2],
            p95: samples[(samples.len() * 95).div_ceil(100).saturating_sub(1)],
        }
    }

    fn emit_benchmark(
        topology: &str,
        chunk_bytes: usize,
        timing: BenchmarkTiming,
        baseline: BenchmarkTiming,
        event_count: usize,
        stats: PipelineStats,
    ) {
        let seconds = timing.median.as_secs_f64();
        let baseline_events_per_second = 100_000.0 / baseline.median.as_secs_f64();
        println!(
            "{topology},{chunk_bytes},{:.3},{:.3},{:.0},{:.3},{},{},{},{}",
            timing.median.as_secs_f64() * 1_000.0,
            timing.p95.as_secs_f64() * 1_000.0,
            event_count as f64 / seconds,
            (event_count as f64 / seconds) / baseline_events_per_second,
            stats.batches,
            stats.max_batch_records,
            stats.max_batch_decoded_payload_bytes,
            stats.queue_full_handoffs,
        );
    }

    #[cfg(feature = "pipeline-dhat")]
    fn run_profile_operation(
        operation: &str,
        input: &str,
        chunk_bytes: usize,
        limits: &TelexLimits,
    ) {
        match operation {
            "one-shot" => {
                let validation = validate_telex_with_limits(black_box(input), &[], limits)
                    .expect("generated Telex syntax");
                assert!(validation.valid, "generated Telex semantics");
                black_box(validation);
            }
            "t1-capacity-1" => {
                let (event_count, stats) =
                    run_incremental_threaded(black_box(input), chunk_bytes, limits);
                assert_eq!(event_count, 100_000);
                black_box(stats);
            }
            _ => unreachable!("profile operation checked before dispatch"),
        }
    }

    fn build_benchmark_telex(event_count: usize) -> String {
        let mut output = String::with_capacity(event_count * 60);
        output.push_str("telex.aes=1\n\npath=$.items\nkind=ListNode\ndatatype=list<string>\n");
        for index in 0..event_count - 1 {
            output.push_str("\npath=$.items[");
            output.push_str(&index.to_string());
            output.push_str("]\n");
            match index % 4 {
                0 => {
                    output.push_str("kind=StringLiteral\nvalue=value-");
                    output.push_str(&index.to_string());
                    output.push_str("-café\n");
                }
                1 => {
                    output.push_str("kind=NumberLiteral\ndatatype=int\nvalue=");
                    output.push_str(&index.to_string());
                    output.push('\n');
                }
                2 => {
                    output.push_str("kind=BooleanLiteral\nvalue=");
                    output.push_str(if index % 8 == 2 { "true\n" } else { "false\n" });
                }
                _ => output.push_str("kind=CloneReference\nvalue=$.items[0]\n"),
            }
        }
        output
    }

    fn read_json(path: &Path) -> Value {
        serde_json::from_str(&fs::read_to_string(path).expect("CTS JSON must be readable"))
            .expect("CTS JSON must parse")
    }

    fn assert_parse_equivalent(
        actual: &Result<ParsedTelex, TelexSyntaxError>,
        expected: &Result<ParsedTelex, TelexSyntaxError>,
        id: &str,
        split: Option<usize>,
    ) {
        if let (Err(actual), Err(expected)) = (actual, expected)
            && actual.counter == Some("max_input_bytes")
            && expected.counter == Some("max_input_bytes")
        {
            assert_eq!(actual.code, expected.code, "{id}: {split:?}");
            assert_eq!(actual.line, expected.line, "{id}: {split:?}");
            assert_eq!(actual.counter, expected.counter, "{id}: {split:?}");
            assert_eq!(actual.limit, expected.limit, "{id}: {split:?}");
            assert!(
                actual.observed.expect("incremental observation")
                    > actual.limit.expect("input limit"),
                "{id}: {split:?}",
            );
            assert!(actual.observed <= expected.observed, "{id}: {split:?}",);
            return;
        }
        assert_eq!(actual, expected, "{id}: {split:?}");
    }

    fn assert_same_input_limit_error(
        actual: &TelexSyntaxError,
        expected: &TelexSyntaxError,
        input_length: usize,
        split: usize,
    ) {
        assert_eq!(actual.code, expected.code, "split at byte {split}");
        assert_eq!(actual.line, expected.line, "split at byte {split}");
        assert_eq!(actual.counter, expected.counter, "split at byte {split}");
        assert_eq!(actual.limit, expected.limit, "split at byte {split}");
        let observed = actual.observed.expect("incremental observation");
        assert!(
            observed > actual.limit.expect("input limit") && observed <= input_length,
            "split at byte {split}: observed {observed}",
        );
    }

    fn cts_limits(vector: &Value) -> TelexLimits {
        let mut limits = TelexLimits::default();
        let Some(values) = vector["input"]["limits"].as_object() else {
            return limits;
        };
        for (name, value) in values {
            let value = value
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .expect("CTS limit must fit usize");
            match name.as_str() {
                "max_input_bytes" => limits.max_input_bytes = value,
                "max_line_bytes" => limits.max_line_bytes = value,
                "max_fields_per_event" => limits.max_fields_per_event = value,
                "max_events" => limits.max_events = value,
                "max_decoded_payload_bytes" => limits.max_decoded_payload_bytes = value,
                "max_path_depth" => limits.max_path_depth = value,
                "max_path_characters" => limits.max_path_characters = value,
                "max_attribute_depth" => limits.max_attribute_depth = value,
                "max_value_nesting_depth" => limits.max_value_nesting_depth = value,
                "max_string_codepoints" => limits.max_string_codepoints = value,
                "max_key_segment_codepoints" => limits.max_key_segment_codepoints = value,
                "max_list_items" => limits.max_list_items = value,
                "max_tuple_items" => limits.max_tuple_items = value,
                "max_generic_depth" => limits.max_generic_depth = value,
                "max_generic_arguments" => limits.max_generic_arguments = value,
                "max_clarifier_values" => limits.max_clarifier_values = value,
                "max_datatype_components" => limits.max_datatype_components = value,
                _ => panic!("unknown CTS limit: {name}"),
            }
        }
        limits
    }
}
