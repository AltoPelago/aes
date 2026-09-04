const VERSION_LINE = 'telex.aes=0';
const PROFILE_FIELD = 'profile';
const PROJECTION_FIELD = 'projection';
const FIELD_NAME = /^[a-z][a-z0-9-]*(?:\.[a-z][a-z0-9-]*)*$/;
const BARE_PATH_MEMBER = /^[A-Za-z_][A-Za-z0-9_]*$/u;
const CORE_FIELD_ORDER = new Map([
  'header',
  'path',
  'kind',
  'datatype',
  'identity',
  'value',
  'origin',
  'span',
].map((field, index) => [field, index]));
const CORE_FIELDS = new Set(CORE_FIELD_ORDER.keys());
const VALUE_KINDS = new Set([
  'string',
  'number',
  'infinity',
  'nan',
  'null',
  'boolean',
  'toggle',
  'hex',
  'radix',
  'encoding',
  'separator',
  'sansa-address',
  'date',
  'time',
  'datetime',
  'wtc',
  'object',
  'list',
  'tuple',
  'node',
  'node-head',
  'clone-reference',
  'pointer-reference',
]);
const VALUELESS_KINDS = new Set(['object', 'list', 'tuple', 'node']);
const INDEX_CONTAINER_KINDS = new Set(['list', 'tuple', 'node-head']);
const EXACT_VALUES = new Map([
  ['infinity', new Set(['Infinity', '-Infinity'])],
  ['nan', new Set(['NaN', '-NaN'])],
  ['boolean', new Set(['true', 'false'])],
  ['toggle', new Set(['yes', 'no', 'on', 'off'])],
]);

export const TELEX_VERSION = '0';
export const COMPLETE_AES_PROFILE = 'aes.complete.v0';
export const PARTIAL_AES_PROFILE = 'aes.partial.v0';
export const AEON_DOCUMENT_PROJECTION = 'aeon.document.v0';

export class TelexSyntaxError extends Error {
  constructor(message, line, code = 'TELEX_SYNTAX_ERROR') {
    super(line === undefined ? message : `Line ${line}: ${message}`);
    this.name = 'TelexSyntaxError';
    this.line = line;
    this.code = code;
  }
}

export function parseTelex(input) {
  if (typeof input !== 'string') {
    throw new TypeError('Telex input must be a string');
  }
  if (input.startsWith('\uFEFF')) {
    throw new TelexSyntaxError('UTF-8 byte-order marks are not allowed', 1, 'TELEX_BOM');
  }
  if (/\r(?!\n)/u.test(input)) {
    throw new TelexSyntaxError('Bare carriage returns are not allowed', undefined, 'TELEX_BARE_CR');
  }

  const canonicalLineEndings = !input.includes('\r\n');
  const normalized = input.replaceAll('\r\n', '\n');
  const hasFinalLf = normalized.endsWith('\n');
  const lines = normalized.split('\n');
  if (hasFinalLf) lines.pop();

  if (lines[0] !== VERSION_LINE) {
    throw new TelexSyntaxError(`Expected ${VERSION_LINE}`, 1, 'TELEX_INVALID_PREAMBLE');
  }

  if (lines.length === 1) {
    return {
      version: TELEX_VERSION,
      profile: COMPLETE_AES_PROFILE,
      profileExplicit: false,
      projection: null,
      projectionExplicit: false,
      records: [],
      canonical: canonicalLineEndings && hasFinalLf,
    };
  }

  let profile = COMPLETE_AES_PROFILE;
  let profileExplicit = false;
  let projection = null;
  let projectionExplicit = false;
  let headerCanonical = true;
  let eventStart = 1;
  let lastHeaderRank = -1;
  while (eventStart < lines.length && lines[eventStart] !== '') {
    const lineNumber = eventStart + 1;
    const delimiter = lines[eventStart].indexOf('=');
    const field = delimiter < 0 ? '' : lines[eventStart].slice(0, delimiter);
    if (field !== PROFILE_FIELD && field !== PROJECTION_FIELD) break;
    const rank = field === PROFILE_FIELD ? 0 : 1;
    if (rank < lastHeaderRank) headerCanonical = false;
    lastHeaderRank = rank;
    const decoded = decodePayload(lines[eventStart].slice(delimiter + 1), lineNumber);
    headerCanonical &&= decoded.canonical;
    if (decoded.value.length === 0) {
      const label = field === PROFILE_FIELD ? 'Profile' : 'Projection';
      const code = field === PROFILE_FIELD ? 'TELEX_EMPTY_PROFILE' : 'TELEX_EMPTY_PROJECTION';
      throw new TelexSyntaxError(`${label} identifier must not be empty`, lineNumber, code);
    }
    if (field === PROFILE_FIELD) {
      if (profileExplicit) {
        throw new TelexSyntaxError('Duplicate stream field: profile', lineNumber, 'TELEX_DUPLICATE_STREAM_FIELD');
      }
      profile = decoded.value;
      profileExplicit = true;
    } else {
      if (projectionExplicit) {
        throw new TelexSyntaxError('Duplicate stream field: projection', lineNumber, 'TELEX_DUPLICATE_STREAM_FIELD');
      }
      projection = decoded.value;
      projectionExplicit = true;
    }
    eventStart += 1;
  }

  if (eventStart === lines.length) {
    return {
      version: TELEX_VERSION,
      profile,
      profileExplicit,
      projection,
      projectionExplicit,
      records: [],
      canonical: canonicalLineEndings && hasFinalLf && headerCanonical,
    };
  }

  if (lines[eventStart] !== '') {
    throw new TelexSyntaxError(
      'Expected a blank line after the stream header',
      eventStart + 1,
      'TELEX_MISSING_HEADER_SEPARATOR',
    );
  }
  eventStart += 1;

  const records = [];
  let record = null;
  let canonical = canonicalLineEndings && hasFinalLf && headerCanonical;
  // Count the required preamble separator so an additional blank is visibly
  // non-canonical.
  let separatorWidth = 1;

  for (let index = eventStart; index < lines.length; index += 1) {
    const lineNumber = index + 1;
    const line = lines[index];
    if (line === '') {
      separatorWidth += 1;
      if (record !== null) {
        records.push(Object.fromEntries(record));
        record = null;
      }
      continue;
    }

    if (separatorWidth > 1) canonical = false;
    separatorWidth = 0;
    if (record === null) record = new Map();

    const delimiter = line.indexOf('=');
    if (delimiter < 1) {
      throw new TelexSyntaxError('Expected field=value', lineNumber, 'TELEX_INVALID_FIELD_LINE');
    }
    const field = line.slice(0, delimiter);
    if (!FIELD_NAME.test(field)) {
      throw new TelexSyntaxError(`Invalid field name: ${field}`, lineNumber, 'TELEX_INVALID_FIELD_NAME');
    }
    if (record.has(field)) {
      throw new TelexSyntaxError(`Duplicate field: ${field}`, lineNumber, 'TELEX_DUPLICATE_FIELD');
    }
    const decoded = decodePayload(line.slice(delimiter + 1), lineNumber);
    canonical &&= decoded.canonical;
    record.set(field, decoded.value);
  }

  if (record !== null) records.push(Object.fromEntries(record));
  if (separatorWidth > 0) canonical = false;
  canonical &&= records.every(hasCanonicalFieldOrder);

  return {
    version: TELEX_VERSION,
    profile,
    profileExplicit,
    projection,
    projectionExplicit,
    records,
    canonical,
  };
}

export function encodeTelex(records, options = {}) {
  if (!Array.isArray(records)) {
    throw new TypeError('Telex records must be an array');
  }
  const { profile, projection } = options;
  if (profile !== undefined && (typeof profile !== 'string' || profile.length === 0)) {
    throw new TypeError('Telex profile must be a non-empty string');
  }
  if (projection !== undefined && (typeof projection !== 'string' || projection.length === 0)) {
    throw new TypeError('Telex projection must be a non-empty string');
  }
  let header = VERSION_LINE;
  if (profile !== undefined) header += `\n${PROFILE_FIELD}=${encodePayload(profile)}`;
  if (projection !== undefined) header += `\n${PROJECTION_FIELD}=${encodePayload(projection)}`;
  if (records.length === 0) return `${header}\n`;

  const stanzas = records.map((record, recordIndex) => {
    const entries = record instanceof Map ? [...record.entries()] : Object.entries(record);
    if (entries.length === 0) {
      throw new TypeError(`Telex record ${recordIndex + 1} must not be empty`);
    }
    for (const [field, value] of entries) {
      if (!FIELD_NAME.test(field)) {
        throw new TypeError(`Invalid Telex field name: ${field}`);
      }
      if (typeof value !== 'string') {
        throw new TypeError(`Telex field ${field} must have a string payload`);
      }
    }
    entries.sort(compareFields);
    return entries.map(([field, value]) => `${field}=${encodePayload(value)}`).join('\n');
  });

  return `${header}\n\n${stanzas.join('\n\n')}\n`;
}

export function canonicalizeTelex(input) {
  const parsed = parseTelex(input);
  const options = {
    ...(parsed.profileExplicit ? { profile: parsed.profile } : {}),
    ...(parsed.projectionExplicit ? { projection: parsed.projection } : {}),
  };
  return encodeTelex(parsed.records, options);
}

/**
 * Parse Telex and report whether every non-root path prefix is present.
 * This is a structural convenience check, not full AES profile validation.
 */
export function checkTelexCompleteness(input) {
  const parsed = parseTelex(input);
  return checkPrefixCompleteness(parsed.records, { projection: parsed.projection });
}

export function checkPrefixCompleteness(records, options = {}) {
  if (!Array.isArray(records)) {
    throw new TypeError('Telex records must be an array');
  }

  const paths = new Map([['path', new Set()], ['header', new Set()]]);
  for (const [index, record] of records.entries()) {
    const addressField = recordAddressField(record);
    if (addressField === null) {
      throw new TypeError(`Telex record ${index + 1} must have exactly one string address field`);
    }
    if (addressField === 'header' && options.projection !== AEON_DOCUMENT_PROJECTION) {
      throw new TypeError(`Telex record ${index + 1} requires projection '${AEON_DOCUMENT_PROJECTION}'`);
    }
    paths.get(addressField).add(record[addressField]);
  }

  const missing = [];
  const reported = new Set();
  for (const record of records) {
    const addressField = recordAddressField(record);
    const address = record[addressField];
    const prefixes = canonicalPathPrefixes(address);
    for (const prefix of prefixes.slice(0, -1)) {
      const reportKey = `${addressField}\0${prefix}`;
      if (paths.get(addressField).has(prefix) || reported.has(reportKey)) continue;
      reported.add(reportKey);
      missing.push({
        ...(addressField === 'header' ? { field: 'header' } : {}),
        path: prefix,
        requiredBy: address,
      });
    }
  }

  return { complete: missing.length === 0, missing };
}

/**
 * Validate decoded events against the selected AES profile.
 * Syntax errors still throw from parseTelex; semantic failures are diagnostics.
 */
export function validateTelex(input, options = {}) {
  const parsed = typeof input === 'string' ? parseTelex(input) : input;
  if (parsed === null || typeof parsed !== 'object' || !Array.isArray(parsed.records)) {
    throw new TypeError('Expected Telex text or a parsed Telex result');
  }
  return validateTelexRecords(parsed.records, {
    ...options,
    profile: parsed.profile ?? COMPLETE_AES_PROFILE,
    projection: parsed.projection ?? null,
  });
}

export function validateTelexRecords(records, options = {}) {
  if (!Array.isArray(records)) {
    throw new TypeError('Telex records must be an array');
  }

  const profile = options.profile ?? COMPLETE_AES_PROFILE;
  const projection = options.projection ?? null;
  if (typeof profile !== 'string' || profile.length === 0) {
    throw new TypeError('AES profile must be a non-empty string');
  }
  if (projection !== null && (typeof projection !== 'string' || projection.length === 0)) {
    throw new TypeError('AES projection must be null or a non-empty string');
  }
  const registeredFields = new Set(options.registeredFields ?? []);
  for (const field of registeredFields) {
    if (typeof field !== 'string' || !FIELD_NAME.test(field) || CORE_FIELDS.has(field)) {
      throw new TypeError(`Invalid registered extension field: ${String(field)}`);
    }
  }

  const diagnostics = [];
  const events = [];
  if (profile !== COMPLETE_AES_PROFILE && profile !== PARTIAL_AES_PROFILE) {
    diagnostics.push(diagnostic(
      'AES_UNSUPPORTED_PROFILE',
      `Unsupported AES profile: ${profile}`,
    ));
  }
  if (projection !== null && projection !== AEON_DOCUMENT_PROJECTION) {
    diagnostics.push(diagnostic(
      'AES_UNSUPPORTED_PROJECTION',
      `Unsupported AES projection: ${projection}`,
    ));
  }

  let bodySeen = false;
  for (let index = 0; index < records.length; index += 1) {
    const source = records[index];
    if (source === null || typeof source !== 'object') {
      diagnostics.push(diagnostic(
        'AES_INVALID_EVENT',
        'An AES event must be an object or Map',
        { record: index },
      ));
      continue;
    }
    const event = source instanceof Map ? Object.fromEntries(source) : source;
    const addressField = recordAddressField(event);
    const address = addressField === null ? undefined : event[addressField];
    const context = { record: index, ...(typeof address === 'string' ? { path: address } : {}) };

    for (const [field, payload] of Object.entries(event)) {
      if (typeof payload !== 'string') {
        diagnostics.push(diagnostic(
          'AES_INVALID_PAYLOAD',
          `Field '${field}' must have a string payload`,
          { ...context, field },
        ));
      }
      if (!CORE_FIELDS.has(field) && !registeredFields.has(field)) {
        diagnostics.push(diagnostic(
          'AES_UNKNOWN_FIELD',
          `Field '${field}' is not registered by profile '${profile}'`,
          { ...context, field },
        ));
      }
    }

    const hasPath = Object.hasOwn(event, 'path');
    const hasHeader = Object.hasOwn(event, 'header');
    if (!hasPath && !hasHeader) {
      diagnostics.push(diagnostic('AES_MISSING_ADDRESS', "AES records require exactly one of 'path' or 'header'", context));
    } else if (hasPath && hasHeader) {
      diagnostics.push(diagnostic('AES_MULTIPLE_ADDRESSES', "AES records cannot carry both 'path' and 'header'", context));
    }
    if (!Object.hasOwn(event, 'kind')) {
      diagnostics.push(diagnostic('AES_MISSING_FIELD', "AES records require 'kind'", { ...context, field: 'kind' }));
    }

    let pathDetails;
    if (typeof address === 'string') {
      try {
        pathDetails = parseCanonicalDataPath(address);
        if (address === '$') {
          throw new TypeError('The root is not an event path');
        }
      } catch (error) {
        diagnostics.push(diagnostic(
          addressField === 'header' ? 'AES_INVALID_HEADER_PATH' : 'AES_INVALID_PATH',
          error.message,
          { ...context, field: addressField ?? 'path' },
        ));
      }
    }
    if (addressField === 'header') {
      if (projection !== AEON_DOCUMENT_PROJECTION) {
        diagnostics.push(diagnostic(
          'AES_HEADER_REQUIRES_PROJECTION',
          `Header records require projection '${AEON_DOCUMENT_PROJECTION}'`,
          { ...context, field: 'header' },
        ));
      }
      if (bodySeen) {
        diagnostics.push(diagnostic(
          'AES_HEADER_ORDER',
          'Header records must precede body events',
          { ...context, field: 'header' },
        ));
      }
      if (pathDetails !== undefined && !isAeonHeaderPath(address, pathDetails)) {
        diagnostics.push(diagnostic(
          'AES_INVALID_HEADER_PATH',
          "Header paths must begin with a quoted 'aeon:' member",
          { ...context, field: 'header' },
        ));
        pathDetails = undefined;
      }
    } else if (addressField === 'path') {
      bodySeen = true;
    }

    const knownKind = typeof event.kind === 'string' && VALUE_KINDS.has(event.kind);
    if (typeof event.kind === 'string' && !knownKind) {
      diagnostics.push(diagnostic(
        'AES_UNKNOWN_KIND',
        `Unknown AES value kind: ${event.kind}`,
        { ...context, field: 'kind' },
      ));
    }

    if (knownKind) validateEventValue(event, index, diagnostics);
    validateOptionalCoreFields(event, index, diagnostics);

    events.push({ event, index, addressField, address, pathDetails, knownKind });
  }

  const bodyEvents = events.filter(({ addressField }) => addressField === 'path');
  const headerEvents = events.filter(({ addressField }) => addressField === 'header');
  if (profile === COMPLETE_AES_PROFILE) {
    validateCompleteStream(bodyEvents, diagnostics);
    validateReferenceTargets(
      [...bodyEvents, ...(projection === AEON_DOCUMENT_PROJECTION ? headerEvents : [])],
      bodyEvents,
      diagnostics,
    );
  }
  if (projection === AEON_DOCUMENT_PROJECTION) {
    validateCompleteStream(headerEvents, diagnostics);
  }
  validateIdentityUniqueness(
    profile === COMPLETE_AES_PROFILE
      ? [...bodyEvents, ...(projection === AEON_DOCUMENT_PROJECTION ? headerEvents : [])]
      : projection === AEON_DOCUMENT_PROJECTION ? headerEvents : [],
    diagnostics,
  );

  return { valid: diagnostics.length === 0, profile, diagnostics };
}

function validateEventValue(event, index, diagnostics) {
  const addressField = recordAddressField(event);
  const address = addressField === null ? undefined : event[addressField];
  const context = { record: index, ...(typeof address === 'string' ? { path: address } : {}) };
  const hasValue = Object.hasOwn(event, 'value');
  if (VALUELESS_KINDS.has(event.kind)) {
    if (hasValue) {
      diagnostics.push(diagnostic(
        'AES_UNEXPECTED_VALUE',
        `Kind '${event.kind}' must not carry 'value'`,
        { ...context, field: 'value' },
      ));
    }
    return;
  }
  if (!hasValue) {
    diagnostics.push(diagnostic(
      'AES_MISSING_VALUE',
      `Kind '${event.kind}' requires 'value'`,
      { ...context, field: 'value' },
    ));
    return;
  }
  if (typeof event.value !== 'string') return;

  const exact = EXACT_VALUES.get(event.kind);
  if (exact !== undefined && !exact.has(event.value)) {
    diagnostics.push(diagnostic(
      'AES_INVALID_VALUE',
      `Invalid '${event.kind}' payload: ${event.value}`,
      { ...context, field: 'value' },
    ));
  }
  if (event.kind === 'hex' && !/^[0-9a-f]+$/u.test(event.value)) {
    diagnostics.push(diagnostic(
      'AES_INVALID_VALUE',
      'Hex payloads require one or more lowercase hexadecimal digits',
      { ...context, field: 'value' },
    ));
  }
  if (event.kind === 'node-head' && event.value.length === 0) {
    diagnostics.push(diagnostic(
      'AES_INVALID_VALUE',
      'Node tags must not be empty',
      { ...context, field: 'value' },
    ));
  }
  if (event.kind === 'wtc') {
    const separator = event.value.lastIndexOf('&');
    const reference = separator === -1 ? undefined : event.value.slice(separator + 1);
    if (reference?.toLowerCase() === 'local' && reference !== 'local') {
      diagnostics.push(diagnostic(
        'AES_INVALID_VALUE',
        "The reserved WTC reference must be exact lowercase 'local'",
        { ...context, field: 'value' },
      ));
    }
  }
  if (event.kind === 'clone-reference' || event.kind === 'pointer-reference') {
    try {
      parseCanonicalDataPath(event.value);
      if (event.value === '$') throw new TypeError('The root is not an event path');
    } catch (error) {
      diagnostics.push(diagnostic(
        'AES_INVALID_REFERENCE',
        error.message,
        { ...context, field: 'value' },
      ));
    }
  }
}

function validateOptionalCoreFields(event, index, diagnostics) {
  const addressField = recordAddressField(event);
  const address = addressField === null ? undefined : event[addressField];
  const context = { record: index, ...(typeof address === 'string' ? { path: address } : {}) };
  for (const field of ['datatype', 'identity']) {
    if (Object.hasOwn(event, field) && event[field] === '') {
      diagnostics.push(diagnostic(
        'AES_EMPTY_FIELD',
        `Field '${field}' must not be empty when present`,
        { ...context, field },
      ));
    }
  }

  if (typeof event.origin === 'string' && !/^sha256:[0-9a-f]{64}$/u.test(event.origin)) {
    diagnostics.push(diagnostic(
      'AES_INVALID_ORIGIN',
      "Origin must be 'sha256:' followed by 64 lowercase hexadecimal digits",
      { ...context, field: 'origin' },
    ));
  }

  if (!Object.hasOwn(event, 'span') || typeof event.span !== 'string') return;
  if (!Object.hasOwn(event, 'origin')) {
    diagnostics.push(diagnostic(
      'AES_SPAN_REQUIRES_ORIGIN',
      "Field 'span' requires source identity in 'origin'",
      { ...context, field: 'span' },
    ));
  }
  const match = event.span.match(/^(0|[1-9][0-9]*):(0|[1-9][0-9]*)$/u);
  if (match === null || BigInt(match[1]) >= BigInt(match[2])) {
    diagnostics.push(diagnostic(
      'AES_INVALID_SPAN',
      "Span must be canonical 'start-byte:end-byte' with start-byte < end-byte",
      { ...context, field: 'span' },
    ));
  }
}

function validateReferenceTargets(referenceEvents, bodyEvents, diagnostics) {
  const bodyPaths = new Set(
    bodyEvents
      .filter(({ pathDetails }) => pathDetails !== undefined)
      .map(({ address }) => address),
  );
  for (const { event, index, address } of referenceEvents) {
    if (event.kind !== 'clone-reference' && event.kind !== 'pointer-reference') continue;
    if (typeof event.value !== 'string' || event.value === '$') continue;
    try {
      parseCanonicalDataPath(event.value);
    } catch {
      continue;
    }
    if (!bodyPaths.has(event.value)) {
      diagnostics.push(diagnostic(
        'AES_MISSING_REFERENCE_TARGET',
        `Missing reference target '${event.value}'`,
        { record: index, path: address, field: 'value', requiredPath: event.value },
      ));
    }
  }
}

function validateCompleteStream(events, diagnostics) {
  const byPath = new Map();

  for (const candidate of events) {
    const { index, address, pathDetails } = candidate;
    if (pathDetails !== undefined) {
      if (byPath.has(address)) {
        diagnostics.push(diagnostic(
          'AES_DUPLICATE_PATH',
          `Duplicate record address '${address}'`,
          { record: index, path: address, firstRecord: byPath.get(address).index },
        ));
      } else {
        byPath.set(address, candidate);
      }
    }

  }

  for (const candidate of events) {
    const { event, index, address, pathDetails } = candidate;
    if (pathDetails === undefined) continue;
    if (pathDetails.segments.length === 1) {
      if (pathDetails.segments[0].type !== 'member') {
        diagnostics.push(diagnostic(
          'AES_MISSING_PARENT',
          "Only a member event can be a direct child of the unrepresented '$' root",
          { record: index, path: address, requiredPath: '$' },
        ));
      }
      if (event.kind === 'node-head') {
        diagnostics.push(invalidNodeHeadPlacement(event, index));
      }
      continue;
    }

    const segment = pathDetails.segments.at(-1);
    const parentPath = pathDetails.prefixes.at(-2);
    const parent = byPath.get(parentPath);
    if (parent === undefined) {
      diagnostics.push(diagnostic(
        'AES_MISSING_PARENT',
        `Missing parent event '${parentPath}'`,
        { record: index, path: address, requiredPath: parentPath },
      ));
      continue;
    }

    if (segment.type === 'member' && parent.event.kind !== 'object') {
      diagnostics.push(incompatibleParent(event, index, parentPath, parent.event.kind, 'object'));
    } else if (segment.type === 'index') {
      if (parent.event.kind === 'node') {
        if (event.kind !== 'node-head') {
          diagnostics.push(incompatibleParent(event, index, parentPath, 'node', 'node-head child'));
        }
      } else if (!INDEX_CONTAINER_KINDS.has(parent.event.kind)) {
        diagnostics.push(incompatibleParent(
          event,
          index,
          parentPath,
          parent.event.kind,
          'list, tuple, node, or node-head',
        ));
      }
    }

    if (event.kind === 'node-head'
      && (segment.type !== 'index' || parent.event.kind !== 'node')) {
      diagnostics.push(invalidNodeHeadPlacement(event, index));
    }
  }
}

function validateIdentityUniqueness(events, diagnostics) {
  const identities = new Map();
  for (const { event, index, address } of events) {
    if (typeof event.identity !== 'string' || event.identity.length === 0) continue;
    if (identities.has(event.identity)) {
      diagnostics.push(diagnostic(
        'AES_DUPLICATE_IDENTITY',
        `Duplicate structural identity '${event.identity}'`,
        { record: index, path: address, field: 'identity', firstRecord: identities.get(event.identity) },
      ));
    } else {
      identities.set(event.identity, index);
    }
  }
}

function incompatibleParent(event, index, parentPath, actual, expected) {
  const addressField = recordAddressField(event);
  const address = addressField === null ? undefined : event[addressField];
  return diagnostic(
    'AES_INCOMPATIBLE_PARENT',
    `Parent '${parentPath}' has kind '${actual}'; expected ${expected}`,
    { record: index, path: address, requiredPath: parentPath },
  );
}

function invalidNodeHeadPlacement(event, index) {
  const addressField = recordAddressField(event);
  const address = addressField === null ? undefined : event[addressField];
  return diagnostic(
    'AES_INVALID_NODE_HEAD',
    "A 'node-head' must be an indexed direct child of a 'node'",
    { record: index, ...(typeof address === 'string' ? { path: address } : {}) },
  );
}

function recordAddressField(record) {
  if (record === null || typeof record !== 'object') return null;
  const hasPath = typeof record.path === 'string';
  const hasHeader = typeof record.header === 'string';
  return hasPath === hasHeader ? null : hasHeader ? 'header' : 'path';
}

function isAeonHeaderPath(path, details) {
  if (details.segments[0]?.type !== 'member') return false;
  const first = details.prefixes[0];
  if (!first?.startsWith('$.[')) return false;
  try {
    const member = JSON.parse(first.slice(3, -1));
    return typeof member === 'string' && member.startsWith('aeon:') && member.length > 5;
  } catch {
    return false;
  }
}

function diagnostic(code, message, context = {}) {
  return { code, message, ...context };
}

function compareFields([left], [right]) {
  const leftRank = CORE_FIELD_ORDER.get(left);
  const rightRank = CORE_FIELD_ORDER.get(right);
  if (leftRank !== undefined || rightRank !== undefined) {
    if (leftRank === undefined) return 1;
    if (rightRank === undefined) return -1;
    return leftRank - rightRank;
  }
  return left < right ? -1 : left > right ? 1 : 0;
}

function hasCanonicalFieldOrder(record) {
  const entries = Object.entries(record);
  const sorted = [...entries].sort(compareFields);
  return entries.every(([field], index) => field === sorted[index][0]);
}

function canonicalPathPrefixes(path) {
  return parseCanonicalDataPath(path).prefixes;
}

function parseCanonicalDataPath(path) {
  if (!path.startsWith('$')) {
    throw new TypeError(`Expected an absolute canonical path: ${path}`);
  }
  if (path === '$') return { prefixes: [], segments: [] };

  const prefixes = [];
  const segments = [];
  let cursor = 1;
  while (cursor < path.length) {
    const start = cursor;
    if (path.startsWith('.@.', cursor)) {
      cursor += 3;
      cursor = readMemberEnd(path, cursor);
      segments.push({ type: 'attribute' });
    } else if (path[cursor] === '.') {
      cursor += 1;
      cursor = readMemberEnd(path, cursor);
      segments.push({ type: 'member' });
    } else if (path[cursor] === '[') {
      const index = path.slice(cursor).match(/^\[(?:0|[1-9][0-9]*)\]/u);
      if (!index) throw new TypeError(`Invalid canonical index in path: ${path}`);
      cursor += index[0].length;
      segments.push({ type: 'index' });
    } else {
      throw new TypeError(`Invalid canonical path segment in: ${path}`);
    }
    prefixes.push(`${prefixes.at(-1) ?? '$'}${path.slice(start, cursor)}`);
  }
  return { prefixes, segments };
}

function readMemberEnd(path, cursor) {
  if (path[cursor] === '[') {
    if (path[cursor + 1] !== '"') {
      throw new TypeError(`Expected a quoted canonical member in path: ${path}`);
    }
    let quoteEnd = cursor + 2;
    let escaped = false;
    for (; quoteEnd < path.length; quoteEnd += 1) {
      const character = path[quoteEnd];
      if (escaped) {
        escaped = false;
      } else if (character === '\\') {
        escaped = true;
      } else if (character === '"') {
        break;
      }
    }
    if (path[quoteEnd] !== '"' || path[quoteEnd + 1] !== ']') {
      throw new TypeError(`Unterminated quoted canonical member in path: ${path}`);
    }
    const encoded = path.slice(cursor + 1, quoteEnd + 1);
    let decoded;
    try {
      decoded = JSON.parse(encoded);
    } catch {
      throw new TypeError(`Invalid quoted canonical member in path: ${path}`);
    }
    if (typeof decoded !== 'string'
      || decoded.length === 0
      || BARE_PATH_MEMBER.test(decoded)
      || hasLoneSurrogate(decoded)
      || JSON.stringify(decoded) !== encoded) {
      throw new TypeError(`Non-canonical quoted member in path: ${path}`);
    }
    return quoteEnd + 2;
  }

  const member = path.slice(cursor).match(/^[A-Za-z_][A-Za-z0-9_]*/u);
  if (!member) throw new TypeError(`Invalid canonical member in path: ${path}`);
  return cursor + member[0].length;
}

function hasLoneSurrogate(value) {
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) return true;
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      return true;
    }
  }
  return false;
}

function decodePayload(payload, lineNumber) {
  let value = '';
  let canonical = true;
  for (let index = 0; index < payload.length; index += 1) {
    const character = payload[index];
    const codePoint = payload.codePointAt(index);
    if (character !== '\\') {
      if (codePoint <= 0x1f || codePoint === 0x7f) {
        throw new TelexSyntaxError(
          'Unescaped control character in payload',
          lineNumber,
          'TELEX_UNESCAPED_CONTROL',
        );
      }
      if (codePoint >= 0xd800 && codePoint <= 0xdfff) {
        throw new TelexSyntaxError(
          'Payload contains a surrogate instead of a Unicode scalar',
          lineNumber,
          'TELEX_INVALID_UNICODE_SCALAR',
        );
      }
      value += String.fromCodePoint(codePoint);
      if (codePoint > 0xffff) index += 1;
      continue;
    }

    const escape = payload[++index];
    if (escape === undefined) {
      throw new TelexSyntaxError('Incomplete escape', lineNumber, 'TELEX_INCOMPLETE_ESCAPE');
    }
    const short = { '\\': '\\', n: '\n', r: '\r', t: '\t', 0: '\0' }[escape];
    if (short !== undefined) {
      value += short;
      continue;
    }
    if (escape !== 'u' || payload[index + 1] !== '{') {
      throw new TelexSyntaxError(`Unknown escape: \\${escape}`, lineNumber, 'TELEX_UNKNOWN_ESCAPE');
    }
    const close = payload.indexOf('}', index + 2);
    if (close === -1) {
      throw new TelexSyntaxError(
        'Unterminated Unicode escape',
        lineNumber,
        'TELEX_UNTERMINATED_UNICODE_ESCAPE',
      );
    }
    const digits = payload.slice(index + 2, close);
    if (!/^[0-9A-Fa-f]{1,6}$/u.test(digits)) {
      throw new TelexSyntaxError('Invalid Unicode escape', lineNumber, 'TELEX_INVALID_UNICODE_ESCAPE');
    }
    const scalar = Number.parseInt(digits, 16);
    if (scalar > 0x10ffff || (scalar >= 0xd800 && scalar <= 0xdfff)) {
      throw new TelexSyntaxError(
        'Unicode escape is not a scalar value',
        lineNumber,
        'TELEX_INVALID_UNICODE_SCALAR',
      );
    }
    if (digits !== scalar.toString(16).toUpperCase()) canonical = false;
    if ({ 0: '\0', 9: '\t', A: '\n', D: '\r' }[digits.toUpperCase()] !== undefined) {
      canonical = false;
    }
    value += String.fromCodePoint(scalar);
    index = close;
  }
  return { value, canonical };
}

function encodePayload(payload) {
  let encoded = '';
  for (const character of payload) {
    const scalar = character.codePointAt(0);
    if (scalar >= 0xd800 && scalar <= 0xdfff) {
      throw new TypeError('Telex payloads must contain Unicode scalar values');
    }
    switch (character) {
      case '\\': encoded += '\\\\'; break;
      case '\n': encoded += '\\n'; break;
      case '\r': encoded += '\\r'; break;
      case '\t': encoded += '\\t'; break;
      case '\0': encoded += '\\0'; break;
      default:
        encoded += scalar < 0x20 || scalar === 0x7f
          ? `\\u{${scalar.toString(16).toUpperCase()}}`
          : character;
    }
  }
  return encoded;
}
