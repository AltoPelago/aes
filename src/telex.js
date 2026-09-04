const VERSION_LINE = 'telex.aes=0';
const FIELD_NAME = /^[a-z][a-z0-9-]*(?:\.[a-z][a-z0-9-]*)*$/;
const CORE_FIELD_ORDER = new Map([
  'path',
  'kind',
  'datatype',
  'identity',
  'value',
  'span',
].map((field, index) => [field, index]));

export const TELEX_VERSION = '0';

export class TelexSyntaxError extends Error {
  constructor(message, line) {
    super(line === undefined ? message : `Line ${line}: ${message}`);
    this.name = 'TelexSyntaxError';
    this.line = line;
  }
}

export function parseTelex(input) {
  if (typeof input !== 'string') {
    throw new TypeError('Telex input must be a string');
  }
  if (input.startsWith('\uFEFF')) {
    throw new TelexSyntaxError('UTF-8 byte-order marks are not allowed', 1);
  }
  if (/\r(?!\n)/u.test(input)) {
    throw new TelexSyntaxError('Bare carriage returns are not allowed');
  }

  const canonicalLineEndings = !input.includes('\r\n');
  const normalized = input.replaceAll('\r\n', '\n');
  const hasFinalLf = normalized.endsWith('\n');
  const lines = normalized.split('\n');
  if (hasFinalLf) lines.pop();

  if (lines[0] !== VERSION_LINE) {
    throw new TelexSyntaxError(`Expected ${VERSION_LINE}`, 1);
  }

  if (lines.length === 1) {
    return {
      version: TELEX_VERSION,
      records: [],
      canonical: canonicalLineEndings && hasFinalLf,
    };
  }
  if (lines[1] !== '') {
    throw new TelexSyntaxError('Expected a blank line after the preamble', 2);
  }

  const records = [];
  let record = null;
  let canonical = canonicalLineEndings && hasFinalLf;
  // Count the required preamble separator so an additional blank is visibly
  // non-canonical.
  let separatorWidth = 1;

  for (let index = 2; index < lines.length; index += 1) {
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
      throw new TelexSyntaxError('Expected field=value', lineNumber);
    }
    const field = line.slice(0, delimiter);
    if (!FIELD_NAME.test(field)) {
      throw new TelexSyntaxError(`Invalid field name: ${field}`, lineNumber);
    }
    if (record.has(field)) {
      throw new TelexSyntaxError(`Duplicate field: ${field}`, lineNumber);
    }
    const decoded = decodePayload(line.slice(delimiter + 1), lineNumber);
    canonical &&= decoded.canonical;
    record.set(field, decoded.value);
  }

  if (record !== null) records.push(Object.fromEntries(record));
  if (separatorWidth > 0) canonical = false;
  canonical &&= records.every(hasCanonicalFieldOrder);

  return { version: TELEX_VERSION, records, canonical };
}

export function encodeTelex(records) {
  if (!Array.isArray(records)) {
    throw new TypeError('Telex records must be an array');
  }
  if (records.length === 0) return `${VERSION_LINE}\n`;

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

  return `${VERSION_LINE}\n\n${stanzas.join('\n\n')}\n`;
}

export function canonicalizeTelex(input) {
  return encodeTelex(parseTelex(input).records);
}

/**
 * Parse Telex and report whether every non-root path prefix is present.
 * This is a structural convenience check, not full AES profile validation.
 */
export function checkTelexCompleteness(input) {
  return checkPrefixCompleteness(parseTelex(input).records);
}

export function checkPrefixCompleteness(records) {
  if (!Array.isArray(records)) {
    throw new TypeError('Telex records must be an array');
  }

  const paths = new Set();
  for (const [index, record] of records.entries()) {
    if (record === null || typeof record !== 'object' || typeof record.path !== 'string') {
      throw new TypeError(`Telex record ${index + 1} must have a string path`);
    }
    paths.add(record.path);
  }

  const missing = [];
  const reported = new Set();
  for (const record of records) {
    const prefixes = canonicalPathPrefixes(record.path);
    for (const prefix of prefixes.slice(0, -1)) {
      if (paths.has(prefix) || reported.has(prefix)) continue;
      reported.add(prefix);
      missing.push({ path: prefix, requiredBy: record.path });
    }
  }

  return { complete: missing.length === 0, missing };
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
  if (!path.startsWith('$')) {
    throw new TypeError(`Expected an absolute canonical path: ${path}`);
  }
  if (path === '$') return [];

  const prefixes = [];
  let cursor = 1;
  while (cursor < path.length) {
    const start = cursor;
    if (path.startsWith('.@.', cursor)) {
      cursor += 3;
      cursor = readMemberEnd(path, cursor);
    } else if (path[cursor] === '.') {
      cursor += 1;
      cursor = readMemberEnd(path, cursor);
    } else if (path[cursor] === '[') {
      const index = path.slice(cursor).match(/^\[(?:0|[1-9][0-9]*)\]/u);
      if (!index) throw new TypeError(`Invalid canonical index in path: ${path}`);
      cursor += index[0].length;
    } else {
      throw new TypeError(`Invalid canonical path segment in: ${path}`);
    }
    prefixes.push(`${prefixes.at(-1) ?? '$'}${path.slice(start, cursor)}`);
  }
  return prefixes;
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
    if (typeof decoded !== 'string' || decoded.length === 0 || JSON.stringify(decoded) !== encoded) {
      throw new TypeError(`Non-canonical quoted member in path: ${path}`);
    }
    return quoteEnd + 2;
  }

  const member = path.slice(cursor).match(/^[A-Za-z_][A-Za-z0-9_]*/u);
  if (!member) throw new TypeError(`Invalid canonical member in path: ${path}`);
  return cursor + member[0].length;
}

function decodePayload(payload, lineNumber) {
  let value = '';
  let canonical = true;
  for (let index = 0; index < payload.length; index += 1) {
    const character = payload[index];
    const codePoint = payload.codePointAt(index);
    if (character !== '\\') {
      if (codePoint <= 0x1f || codePoint === 0x7f) {
        throw new TelexSyntaxError('Unescaped control character in payload', lineNumber);
      }
      if (codePoint >= 0xd800 && codePoint <= 0xdfff) {
        throw new TelexSyntaxError('Payload contains a surrogate instead of a Unicode scalar', lineNumber);
      }
      value += String.fromCodePoint(codePoint);
      if (codePoint > 0xffff) index += 1;
      continue;
    }

    const escape = payload[++index];
    if (escape === undefined) {
      throw new TelexSyntaxError('Incomplete escape', lineNumber);
    }
    const short = { '\\': '\\', n: '\n', r: '\r', t: '\t', 0: '\0' }[escape];
    if (short !== undefined) {
      value += short;
      continue;
    }
    if (escape !== 'u' || payload[index + 1] !== '{') {
      throw new TelexSyntaxError(`Unknown escape: \\${escape}`, lineNumber);
    }
    const close = payload.indexOf('}', index + 2);
    if (close === -1) {
      throw new TelexSyntaxError('Unterminated Unicode escape', lineNumber);
    }
    const digits = payload.slice(index + 2, close);
    if (!/^[0-9A-Fa-f]{1,6}$/u.test(digits)) {
      throw new TelexSyntaxError('Invalid Unicode escape', lineNumber);
    }
    const scalar = Number.parseInt(digits, 16);
    if (scalar > 0x10ffff || (scalar >= 0xd800 && scalar <= 0xdfff)) {
      throw new TelexSyntaxError('Unicode escape is not a scalar value', lineNumber);
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
