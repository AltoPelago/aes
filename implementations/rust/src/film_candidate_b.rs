//! Experimental stateful Film comparator.
//!
//! Candidate B replaces each complete inline address with the longest UTF-8
//! prefix shared with the previous address in the same address plane plus an
//! inline suffix. It deliberately gives up Candidate A's record independence
//! and therefore uses a distinct non-v1 preamble.

use crate::TelexLimits;
use crate::film::{
    FILM_V1_PREAMBLE, FilmError, FilmLimits, FilmStream, decode_film_with_limits,
    encode_film_with_limits,
};

pub const FILM_CANDIDATE_B_PREAMBLE: [u8; 5] = [0x4f, 0x5f, 0x42, 0xff, 0x00];

const HEADER_PLANE: u8 = 0x01;

pub fn encode_film_candidate_b(
    stream: &FilmStream,
    registered_fields: &[&str],
) -> Result<Vec<u8>, FilmError> {
    encode_film_candidate_b_with_limits(
        stream,
        registered_fields,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
}

pub fn encode_film_candidate_b_with_limits(
    stream: &FilmStream,
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<Vec<u8>, FilmError> {
    let candidate_a = encode_film_with_limits(stream, registered_fields, film_limits, aes_limits)?;
    candidate_a_to_b(&candidate_a, film_limits)
}

pub fn decode_film_candidate_b(
    input: &[u8],
    registered_fields: &[&str],
) -> Result<FilmStream, FilmError> {
    decode_film_candidate_b_with_limits(
        input,
        registered_fields,
        &FilmLimits::default(),
        &TelexLimits::default(),
    )
}

pub fn decode_film_candidate_b_with_limits(
    input: &[u8],
    registered_fields: &[&str],
    film_limits: &FilmLimits,
    aes_limits: &TelexLimits,
) -> Result<FilmStream, FilmError> {
    let candidate_a = candidate_b_to_a(input, film_limits)?;
    decode_film_with_limits(&candidate_a, registered_fields, film_limits, aes_limits)
}

fn candidate_a_to_b(input: &[u8], limits: &FilmLimits) -> Result<Vec<u8>, FilmError> {
    check_limit(
        "max_input_bytes",
        input.len(),
        limits.max_input_bytes,
        0,
        None,
        "stream",
    )?;
    let mut reader = Cursor::new(input);
    reader.expect_preamble(&FILM_V1_PREAMBLE, "candidate-a-preamble")?;
    let context_end = reader.read_context_end(limits)?;
    let mut output = buffer_with_capacity(input.len(), None, "stream")?;
    output.extend_from_slice(&FILM_CANDIDATE_B_PREAMBLE);
    output.extend_from_slice(&input[FILM_V1_PREAMBLE.len()..context_end]);

    let mut previous_body = Vec::new();
    let mut previous_header = Vec::new();
    let mut record_index = 0_usize;
    while !reader.is_empty() {
        reader.record = Some(record_index);
        let payload_length = reader.read_limited_length(
            "max_record_bytes",
            limits.max_record_bytes,
            "record-length",
            "record",
        )?;
        if payload_length == 0 {
            return Err(error(
                "FILM_COMPARATOR_INVALID_SOURCE",
                reader.position,
                Some(record_index),
                "record-length",
                "Candidate A record length must be positive",
            ));
        }
        let payload_offset = reader.position;
        let payload = reader.read_exact(payload_length, "record")?;
        let mut record = Cursor::with_base(payload, payload_offset, Some(record_index));
        let control = record.read_byte("record-control")?;
        let kind = record.read_byte("kind")?;
        let address = record.read_string_bytes(limits, "address")?;
        let previous = if control & HEADER_PLANE == 0 {
            &mut previous_body
        } else {
            &mut previous_header
        };
        let prefix_length = common_utf8_prefix_bytes(previous, address)?;
        let suffix = address.get(prefix_length..).ok_or_else(|| {
            error(
                "FILM_COMPARATOR_INTERNAL",
                payload_offset,
                Some(record_index),
                "address",
                "Address prefix was not a byte boundary",
            )
        })?;

        let compressed_size = checked_size(
            checked_size(
                checked_size(
                    2,
                    uleb_width(usize_to_u64(prefix_length, payload_offset, record_index)?),
                    payload_offset,
                    Some(record_index),
                    "record",
                )?,
                encoded_string_size(suffix.len(), payload_offset, Some(record_index))?,
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
        check_limit(
            "max_buffered_bytes",
            compressed_size,
            limits.max_buffered_bytes,
            payload_offset,
            Some(record_index),
            "record",
        )?;
        check_limit(
            "max_buffered_bytes",
            address.len(),
            limits.max_buffered_bytes,
            payload_offset,
            Some(record_index),
            "previous-address",
        )?;

        let mut compressed = buffer_with_capacity(compressed_size, Some(record_index), "record")?;
        compressed.push(control);
        compressed.push(kind);
        push_uleb(
            &mut compressed,
            usize_to_u64(prefix_length, payload_offset, record_index)?,
        );
        push_string_bytes(
            &mut compressed,
            suffix,
            limits,
            Some(record_index),
            "address-suffix",
        )?;
        compressed.extend_from_slice(record.remaining());
        debug_assert_eq!(compressed.len(), compressed_size);
        let frame_size = framed_size(compressed_size, payload_offset, record_index)?;
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
        reserve_for_append(&mut output, frame_size, Some(record_index), "stream")?;
        push_uleb(
            &mut output,
            usize_to_u64(compressed.len(), payload_offset, record_index)?,
        );
        output.extend_from_slice(&compressed);
        drop(compressed);
        previous.clear();
        reserve_for_append(
            previous,
            address.len(),
            Some(record_index),
            "previous-address",
        )?;
        previous.extend_from_slice(address);
        record_index = record_index.saturating_add(1);
    }
    Ok(output)
}

fn candidate_b_to_a(input: &[u8], limits: &FilmLimits) -> Result<Vec<u8>, FilmError> {
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
    let mut output = buffer_with_capacity(input.len(), None, "expanded-stream")?;
    output.extend_from_slice(&FILM_V1_PREAMBLE);
    output.extend_from_slice(&input[FILM_CANDIDATE_B_PREAMBLE.len()..context_end]);

    let mut previous_body = Vec::new();
    let mut previous_header = Vec::new();
    let mut record_index = 0_usize;
    while !reader.is_empty() {
        reader.record = Some(record_index);
        let payload_length = reader.read_limited_length(
            "max_record_bytes",
            limits.max_record_bytes,
            "record-length",
            "record",
        )?;
        if payload_length == 0 {
            return Err(error(
                "FILM_COMPARATOR_NONCANONICAL",
                reader.position,
                Some(record_index),
                "record-length",
                "Candidate B record length must be positive",
            ));
        }
        let payload_offset = reader.position;
        let payload = reader.read_exact(payload_length, "record")?;
        let mut record = Cursor::with_base(payload, payload_offset, Some(record_index));
        let control = record.read_byte("record-control")?;
        let kind = record.read_byte("kind")?;
        let prefix_offset = record.absolute_position();
        let previous = if control & HEADER_PLANE == 0 {
            &mut previous_body
        } else {
            &mut previous_header
        };
        let prefix_value = record.read_uleb("address-prefix")?;
        let suffix = record.read_string_bytes(limits, "address-suffix")?;
        if prefix_value > u64::try_from(previous.len()).unwrap_or(u64::MAX) {
            return Err(error(
                "FILM_COMPARATOR_INVALID_PREFIX",
                prefix_offset,
                Some(record_index),
                "address-prefix",
                "Address prefix exceeds the previous address or splits UTF-8",
            ));
        }
        let prefix_length = usize::try_from(prefix_value).map_err(|_| {
            error(
                "FILM_INTEGER_OVERFLOW",
                prefix_offset,
                Some(record_index),
                "address-prefix",
                "Address prefix length exceeds the host address space",
            )
        })?;
        if !is_char_boundary(previous, prefix_length) {
            return Err(error(
                "FILM_COMPARATOR_INVALID_PREFIX",
                prefix_offset,
                Some(record_index),
                "address-prefix",
                "Address prefix exceeds the previous address or splits UTF-8",
            ));
        }
        let address_length = prefix_length.checked_add(suffix.len()).ok_or_else(|| {
            error(
                "FILM_INTEGER_OVERFLOW",
                prefix_offset,
                Some(record_index),
                "address",
                "Reconstructed address length overflow",
            )
        })?;
        check_limit(
            "max_field_bytes",
            address_length,
            limits.max_field_bytes,
            prefix_offset,
            Some(record_index),
            "address",
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
            limits.max_record_bytes,
            payload_offset,
            Some(record_index),
            "expanded-record",
        )?;
        check_limit(
            "max_buffered_bytes",
            expanded_size,
            limits.max_buffered_bytes,
            payload_offset,
            Some(record_index),
            "expanded-record",
        )?;

        let mut address = buffer_with_capacity(address_length, Some(record_index), "address")?;
        address.extend_from_slice(&previous[..prefix_length]);
        address.extend_from_slice(suffix);
        std::str::from_utf8(&address).map_err(|invalid| {
            error(
                "FILM_INVALID_UTF8",
                prefix_offset.saturating_add(invalid.valid_up_to()),
                Some(record_index),
                "address",
                "Reconstructed address is not UTF-8",
            )
        })?;
        let canonical_prefix = common_utf8_prefix_bytes(previous, &address)?;
        if prefix_length != canonical_prefix {
            return Err(error(
                "FILM_COMPARATOR_NONCANONICAL",
                prefix_offset,
                Some(record_index),
                "address-prefix",
                "Candidate B requires the longest shared UTF-8 address prefix",
            ));
        }

        let mut expanded =
            buffer_with_capacity(expanded_size, Some(record_index), "expanded-record")?;
        expanded.push(control);
        expanded.push(kind);
        push_string_bytes(
            &mut expanded,
            &address,
            limits,
            Some(record_index),
            "address",
        )?;
        expanded.extend_from_slice(record.remaining());
        debug_assert_eq!(expanded.len(), expanded_size);
        let frame_size = framed_size(expanded_size, payload_offset, record_index)?;
        let projected_size = checked_size(
            output.len(),
            frame_size,
            payload_offset,
            Some(record_index),
            "expanded-stream",
        )?;
        check_limit(
            "max_input_bytes",
            projected_size,
            limits.max_input_bytes,
            output.len(),
            Some(record_index),
            "expanded-stream",
        )?;
        reserve_for_append(
            &mut output,
            frame_size,
            Some(record_index),
            "expanded-stream",
        )?;
        push_uleb(
            &mut output,
            usize_to_u64(expanded.len(), payload_offset, record_index)?,
        );
        output.extend_from_slice(&expanded);
        drop(expanded);
        *previous = address;
        record_index = record_index.saturating_add(1);
    }
    Ok(output)
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

fn framed_size(payload_size: usize, offset: usize, record: usize) -> Result<usize, FilmError> {
    let length = usize_to_u64(payload_size, offset, record)?;
    checked_size(
        uleb_width(length),
        payload_size,
        offset,
        Some(record),
        "record",
    )
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

fn reserve_for_append(
    output: &mut Vec<u8>,
    additional: usize,
    record: Option<usize>,
    component: &'static str,
) -> Result<(), FilmError> {
    output.try_reserve_exact(additional).map_err(|_| {
        error(
            "FILM_INTEGER_OVERFLOW",
            output.len(),
            record,
            component,
            "Film comparator output cannot grow in the host size domain",
        )
    })
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
