import {
  COMPLETE_AES_PROFILE,
  validateTelexRecords,
} from './telex.js';
import { normalizeTelexLimits } from './limits.js';

export const FILM_VERSION = '1';
export const FILM_V1_PREAMBLE = Object.freeze([0x4f, 0x5f, 0x5f, 0xff, 0x01]);

export const DEFAULT_FILM_LIMITS = Object.freeze({
  maxInputBytes: 67_108_864,
  maxRecordBytes: 16_777_216,
  maxFieldBytes: 4_194_304,
  maxBufferedBytes: 16_777_216,
});

const CONTEXT_PROFILE = 0x01;
const CONTEXT_PROJECTION = 0x02;
const RECORD_HEADER = 0x01;
const RECORD_DATATYPE = 0x02;
const RECORD_IDENTITY = 0x04;
const RECORD_ORIGIN = 0x08;
const RECORD_SPAN = 0x10;

const KIND_NAMES = Object.freeze([
  'StringLiteral',
  'NumberLiteral',
  'InfinityLiteral',
  'NaNLiteral',
  'NullLiteral',
  'BooleanLiteral',
  'ToggleLiteral',
  'HexLiteral',
  'RadixLiteral',
  'EncodingLiteral',
  'SeparatorLiteral',
  'SansaAddressLiteral',
  'DateLiteral',
  'TimeLiteral',
  'DateTimeLiteral',
  'WTCDateTimeLiteral',
  'ObjectNode',
  'ListNode',
  'TupleLiteral',
  'NodeLiteral',
  'NodeHead',
  'CloneReference',
  'PointerReference',
]);

const VALUELESS_KINDS = new Set([
  'ObjectNode',
  'ListNode',
  'TupleLiteral',
  'NodeLiteral',
]);
const EXTENSION_NAME = /^x\.[a-z][a-z0-9-]*(?:\.[a-z][a-z0-9-]*)+$/u;
const UTF8 = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true });
const MAX_SAFE_BIGINT = BigInt(Number.MAX_SAFE_INTEGER);

export class FilmDecodeError extends Error {
  constructor(code, message, details = {}) {
    super(message);
    this.name = 'FilmDecodeError';
    this.code = code;
    this.offset = details.offset ?? 0;
    this.record = details.record ?? null;
    this.component = details.component ?? 'stream';
    this.stage = details.stage ?? 'film';
    this.diagnostics = details.diagnostics ?? [];
  }
}

/** Normalize the Film-local limits selected by the consumer. */
export function normalizeFilmLimits(options = {}) {
  const source = options.filmLimits ?? options.limits ?? options;
  return Object.freeze({
    maxInputBytes: limit(source, 'maxInputBytes', DEFAULT_FILM_LIMITS.maxInputBytes),
    maxRecordBytes: limit(source, 'maxRecordBytes', DEFAULT_FILM_LIMITS.maxRecordBytes),
    maxFieldBytes: limit(source, 'maxFieldBytes', DEFAULT_FILM_LIMITS.maxFieldBytes),
    maxBufferedBytes: limit(source, 'maxBufferedBytes', DEFAULT_FILM_LIMITS.maxBufferedBytes),
  });
}

/**
 * Decode canonical Film framing and records into owned JavaScript values.
 * The result is provisional: portable AES semantics have not yet been
 * accepted. Use decodeFilm for replay, ingestion, or mutation inputs.
 */
export function decodeFilmSyntax(input, options = {}) {
  assertByteInput(input);
  const filmLimits = normalizeFilmLimits(options);
  const aesLimits = normalizeAesLimits(options);
  enforceFilmLimit('max_input_bytes', input.byteLength, filmLimits.maxInputBytes, {
    component: 'stream',
  });

  if (input.byteLength < FILM_V1_PREAMBLE.length) {
    throw filmError('FILM_TRUNCATED', input.byteLength, null, 'preamble',
      'Film input ended inside the v1 preamble');
  }
  for (let index = 0; index < FILM_V1_PREAMBLE.length; index += 1) {
    if (input[index] !== FILM_V1_PREAMBLE[index]) {
      throw filmError('FILM_INVALID_PREAMBLE', 0, null, 'preamble',
        'Expected O__ FF 01');
    }
  }

  const reader = new Reader(input, 0, null);
  reader.position = FILM_V1_PREAMBLE.length;
  const context = reader.readByte('stream-context');
  if ((context & ~0x03) !== 0) {
    throw filmError('FILM_INVALID_CONTEXT', reader.absolutePosition() - 1, null,
      'stream-context', 'Reserved stream-context bits must be zero');
  }

  const profileExplicit = (context & CONTEXT_PROFILE) !== 0;
  const projectionExplicit = (context & CONTEXT_PROJECTION) !== 0;
  const profile = profileExplicit
    ? reader.readNonEmptyContextString(filmLimits, 'profile')
    : COMPLETE_AES_PROFILE;
  const projection = projectionExplicit
    ? reader.readNonEmptyContextString(filmLimits, 'projection')
    : null;

  const records = [];
  while (!reader.atEnd()) {
    const recordIndex = records.length;
    if (recordIndex >= aesLimits.maxEvents) {
      throw aesLimitError(
        'max_events',
        recordIndex + 1,
        aesLimits.maxEvents,
        reader.absolutePosition(),
        recordIndex,
        'record',
      );
    }
    reader.record = recordIndex;
    const payloadLength = reader.readLength(filmLimits, 'record-length', false);
    if (payloadLength === 0) {
      throw filmError('FILM_NONCANONICAL', reader.absolutePosition(), recordIndex,
        'record-length', 'Film record payload length must be positive');
    }
    enforceFilmLimit('max_record_bytes', payloadLength, filmLimits.maxRecordBytes, {
      offset: reader.absolutePosition(),
      record: recordIndex,
      component: 'record',
    });
    const payloadOffset = reader.absolutePosition();
    const payload = reader.readExact(payloadLength, 'record');
    records.push(decodeRecord(payload, payloadOffset, recordIndex, filmLimits, aesLimits));
  }

  return {
    profile,
    profileExplicit,
    projection,
    projectionExplicit,
    records,
  };
}

/** Decode Film and return an owned stream only after complete AES validation. */
export function decodeFilm(input, options = {}) {
  const stream = decodeFilmSyntax(input, options);
  const validation = validateTelexRecords(stream.records, {
    profile: stream.profile,
    projection: stream.projection,
    registeredFields: options.registeredFields ?? [],
    limits: normalizeAesLimits(options),
  });
  if (!validation.valid) {
    const first = validation.diagnostics[0];
    throw new FilmDecodeError(
      'FILM_AES_INVALID',
      first?.message ?? 'Portable AES validation failed',
      {
        offset: input.byteLength,
        record: first?.record ?? null,
        component: 'aes-events',
        stage: 'aes',
        diagnostics: validation.diagnostics,
      },
    );
  }
  return stream;
}

export function filmV1IsDraft() {
  return true;
}

function decodeRecord(payload, payloadOffset, recordIndex, filmLimits, aesLimits) {
  const reader = new Reader(payload, payloadOffset, recordIndex);
  const control = reader.readByte('record-control');
  if ((control & ~0x1f) !== 0) {
    throw filmError('FILM_INVALID_RECORD', payloadOffset, recordIndex, 'record-control',
      'Reserved record-control bits must be zero');
  }
  if ((control & RECORD_SPAN) !== 0 && (control & RECORD_ORIGIN) === 0) {
    throw filmError('FILM_INVALID_RECORD', payloadOffset, recordIndex, 'record-control',
      'Span presence requires origin presence');
  }

  const kindOffset = reader.absolutePosition();
  const kindCode = reader.readByte('kind');
  const kind = KIND_NAMES[kindCode - 1];
  if (kind === undefined) {
    throw filmError('FILM_INVALID_KIND', kindOffset, recordIndex, 'kind',
      'Unassigned Film v1 kind code');
  }

  const address = reader.readString(filmLimits, 'address');
  const record = {
    [(control & RECORD_HEADER) === 0 ? 'path' : 'header']: address,
    kind,
  };

  if ((control & RECORD_DATATYPE) !== 0) {
    const datatype = reader.readDescriptor(filmLimits, aesLimits, 0);
    record.datatype = datatype.datatype;
    record.generics = datatype.generics;
    record.clarifiers = datatype.clarifiers;
  }
  if ((control & RECORD_IDENTITY) !== 0) {
    record.identity = reader.readString(filmLimits, 'identity');
  }
  if (!VALUELESS_KINDS.has(kind)) {
    record.value = reader.readString(filmLimits, 'value');
  }
  if ((control & RECORD_ORIGIN) !== 0) {
    record.origin = `sha256:${lowerHex(reader.readExact(32, 'origin'))}`;
  }
  if ((control & RECORD_SPAN) !== 0) {
    const start = reader.readUleb('span-start');
    const end = reader.readUleb('span-end');
    if (start >= end) {
      throw filmError('FILM_INVALID_RECORD', reader.absolutePosition(), recordIndex, 'span',
        'Film span requires start < end');
    }
    record.span = `${start}:${end}`;
  }

  let previousExtension = null;
  while (!reader.atEnd()) {
    const nameOffset = reader.absolutePosition();
    const name = reader.readString(filmLimits, 'extension-name');
    if (!EXTENSION_NAME.test(name)) {
      throw filmError('FILM_INVALID_EXTENSION', nameOffset, recordIndex, 'extension-name',
        `Invalid Film extension name: ${name}`);
    }
    if (previousExtension !== null && previousExtension >= name) {
      throw filmError('FILM_NONCANONICAL', nameOffset, recordIndex, 'extension-name',
        'Extensions must be strictly ordered without duplicates');
    }
    record[name] = reader.readString(filmLimits, 'extension-value');
    previousExtension = name;
  }

  return record;
}

class Reader {
  constructor(bytes, baseOffset, record) {
    this.bytes = bytes;
    this.baseOffset = baseOffset;
    this.record = record;
    this.position = 0;
  }

  atEnd() {
    return this.position === this.bytes.byteLength;
  }

  absolutePosition() {
    return this.baseOffset + this.position;
  }

  readByte(component) {
    const byte = this.bytes[this.position];
    if (byte === undefined) {
      throw filmError('FILM_TRUNCATED', this.absolutePosition(), this.record, component,
        'Film input ended before the required byte');
    }
    this.position += 1;
    return byte;
  }

  readExact(length, component) {
    const end = this.position + length;
    if (!Number.isSafeInteger(end)) {
      throw filmError('FILM_INTEGER_OVERFLOW', this.absolutePosition(), this.record, component,
        'Byte range overflow');
    }
    if (end > this.bytes.byteLength) {
      throw filmError('FILM_TRUNCATED', this.absolutePosition(), this.record, component,
        'Film input ended inside a declared field or record');
    }
    const value = this.bytes.subarray(this.position, end);
    this.position = end;
    return value;
  }

  readUleb(component) {
    const start = this.absolutePosition();
    let value = 0n;
    for (let index = 0; index < 10; index += 1) {
      const byte = this.readByte(component);
      const payload = BigInt(byte & 0x7f);
      if (index === 9 && payload > 1n) {
        throw filmError('FILM_INTEGER_OVERFLOW', start, this.record, component,
          'Unsigned LEB128 value exceeds u64');
      }
      value |= payload << BigInt(index * 7);
      if ((byte & 0x80) === 0) {
        if (ulebWidth(value) !== index + 1) {
          throw filmError('FILM_NONCANONICAL', start, this.record, component,
            'Unsigned LEB128 must use its shortest representation');
        }
        return value;
      }
    }
    throw filmError('FILM_INTEGER_OVERFLOW', start, this.record, component,
      'Unsigned LEB128 exceeds ten bytes');
  }

  readLength(filmLimits, component, fieldLength = true) {
    const offset = this.absolutePosition();
    const value = this.readUleb(component);
    if (value > MAX_SAFE_BIGINT) {
      throw filmError('FILM_INTEGER_OVERFLOW', offset, this.record, component,
        'Film length does not fit the JavaScript safe-integer address space');
    }
    const length = Number(value);
    if (fieldLength) {
      enforceFilmLimit('max_field_bytes', length, filmLimits.maxFieldBytes, {
        offset,
        record: this.record,
        component,
      });
    }
    return length;
  }

  readCount(component) {
    const offset = this.absolutePosition();
    const value = this.readUleb(component);
    if (value > MAX_SAFE_BIGINT) {
      throw filmError('FILM_INTEGER_OVERFLOW', offset, this.record, component,
        'Film count does not fit the JavaScript safe-integer domain');
    }
    return Number(value);
  }

  readString(filmLimits, component) {
    const offset = this.absolutePosition();
    const length = this.readLength(filmLimits, component);
    const bytes = this.readExact(length, component);
    try {
      return UTF8.decode(bytes);
    } catch {
      throw filmError('FILM_INVALID_UTF8', offset, this.record, component,
        'Film strings must contain valid UTF-8');
    }
  }

  readNonEmptyContextString(filmLimits, component) {
    const offset = this.absolutePosition();
    const value = this.readString(filmLimits, component);
    if (value.length === 0) {
      throw filmError('FILM_INVALID_CONTEXT', offset, this.record, component,
        'Film stream-context strings must not be empty');
    }
    return value;
  }

  readDescriptor(filmLimits, aesLimits, depth) {
    if (depth > aesLimits.maxGenericDepth) {
      throw aesLimitError('max_generic_depth', depth, aesLimits.maxGenericDepth,
        this.absolutePosition(), this.record, 'datatype');
    }
    const length = this.readLength(filmLimits, 'datatype');
    const offset = this.absolutePosition();
    const body = this.readExact(length, 'datatype');
    const descriptor = new Reader(body, offset, this.record);
    const datatypeOffset = descriptor.absolutePosition();
    const datatype = descriptor.readString(filmLimits, 'datatype-name');
    if (datatype.length === 0) {
      throw filmError('FILM_INVALID_DATATYPE', datatypeOffset, this.record, 'datatype-name',
        'Film datatype names must not be empty');
    }

    const genericCount = descriptor.readCount('generic-count');
    if (genericCount > aesLimits.maxGenericArguments) {
      throw aesLimitError('max_generic_arguments', genericCount, aesLimits.maxGenericArguments,
        descriptor.absolutePosition(), this.record, 'generic-count');
    }
    const generics = [];
    for (let index = 0; index < genericCount; index += 1) {
      const tagOffset = descriptor.absolutePosition();
      const tag = descriptor.readByte('generic-tag');
      if (tag === 0x00) {
        generics.push(descriptor.readDescriptor(filmLimits, aesLimits, depth + 1));
      } else if (tag === 0x02) {
        generics.push({
          kind: 'NumberLiteral',
          value: descriptor.readString(filmLimits, 'generic-number'),
        });
      } else {
        throw filmError('FILM_INVALID_DATATYPE', tagOffset, this.record, 'generic-tag',
          'Film generic tag must be 00 or 02');
      }
    }

    const clarifierCount = descriptor.readCount('clarifier-count');
    if (clarifierCount > aesLimits.maxClarifierValues) {
      throw aesLimitError('max_clarifier_values', clarifierCount, aesLimits.maxClarifierValues,
        descriptor.absolutePosition(), this.record, 'clarifier-count');
    }
    const clarifiers = [];
    for (let index = 0; index < clarifierCount; index += 1) {
      const tagOffset = descriptor.absolutePosition();
      const tag = descriptor.readByte('clarifier-tag');
      if (tag !== 0x01 && tag !== 0x02) {
        throw filmError('FILM_INVALID_DATATYPE', tagOffset, this.record, 'clarifier-tag',
          'Film clarifier tag must be 01 or 02');
      }
      clarifiers.push({
        kind: tag === 0x01 ? 'StringLiteral' : 'NumberLiteral',
        value: descriptor.readString(filmLimits, 'clarifier-value'),
      });
    }
    if (!descriptor.atEnd()) {
      throw filmError('FILM_NONCANONICAL', descriptor.absolutePosition(), this.record, 'datatype',
        'Datatype descriptor length was not consumed exactly');
    }
    return { datatype, generics, clarifiers };
  }
}

function normalizeAesLimits(options) {
  return normalizeTelexLimits({ limits: options.aesLimits ?? options.limits ?? {} });
}

function limit(source, name, fallback) {
  const value = source[name] ?? fallback;
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new TypeError(`${name} must be a non-negative safe integer`);
  }
  return value;
}

function enforceFilmLimit(counter, observed, selected, details) {
  if (observed <= selected) return;
  throw new FilmDecodeError(
    'FILM_LIMIT_EXCEEDED',
    `${counter} observed ${observed}, limit ${selected}`,
    details,
  );
}

function aesLimitError(counter, observed, selected, offset, record, component) {
  const diagnostic = {
    code: 'AES_LIMIT_EXCEEDED',
    message: `${counter} observed value ${observed} exceeds configured limit ${selected}`,
    counter,
    observed,
    limit: selected,
  };
  return new FilmDecodeError('FILM_AES_INVALID', diagnostic.message, {
    offset,
    record,
    component,
    stage: 'aes',
    diagnostics: [diagnostic],
  });
}

function filmError(code, offset, record, component, message) {
  return new FilmDecodeError(code, message, { offset, record, component });
}

function assertByteInput(input) {
  if (!(input instanceof Uint8Array)) {
    throw new TypeError('Film input must be a Uint8Array');
  }
}

function ulebWidth(value) {
  let remaining = value;
  let width = 1;
  while (remaining >= 0x80n) {
    remaining >>= 7n;
    width += 1;
  }
  return width;
}

function lowerHex(bytes) {
  let output = '';
  for (const byte of bytes) output += byte.toString(16).padStart(2, '0');
  return output;
}
