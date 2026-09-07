import { Buffer } from 'node:buffer';
import { createHash } from 'node:crypto';
import { TextDecoder } from 'node:util';

export const AES_SOURCE_BACKED_PREPARATION = 'aes.preparation.source-backed.v0';

const ORIGIN = /^sha256:[0-9a-f]{64}$/u;
const SPAN = /^(0|[1-9][0-9]*):(0|[1-9][0-9]*)$/u;
const UTF8 = new TextDecoder('utf-8', { fatal: true });

/**
 * Audit record-local AES provenance against exact, caller-retained bytes.
 * Artifact lookup may be a Map, a null-prototype/plain object, or a synchronous
 * `(origin) => Uint8Array | null` resolver. Strings are deliberately rejected:
 * callers must make the byte encoding explicit.
 */
export function auditAesSourceProvenance(records, artifacts, options = {}) {
  if (!Array.isArray(records)) {
    return {
      valid: false,
      complete: false,
      verifiedOrigins: [],
      missingOrigins: [],
      diagnostics: [diagnostic('AES_SOURCE_AUDIT_INVALID', 'Source-backed audit requires an ordered record list.')],
    };
  }

  const requireAllRecords = options.requireAllRecords === true;
  const requireAvailable = options.requireAvailable === true;
  const states = new Map();
  const missing = new Set();
  const diagnostics = [];
  let complete = true;

  records.forEach((record, index) => {
    if (!plainRecord(record) || typeof record.origin !== 'string') {
      if (requireAllRecords) {
        complete = false;
        diagnostics.push(diagnostic(
          'AES_SOURCE_REQUIRED',
          'Source-backed preparation requires origin on every payload record.',
          index,
          'origin',
        ));
      }
      return;
    }
    const origin = record.origin;
    if (!ORIGIN.test(origin)) {
      diagnostics.push(diagnostic('AES_INVALID_ORIGIN', 'Origin is not a canonical AES v0 source digest.', index, 'origin'));
      return;
    }

    let state = states.get(origin);
    if (state === undefined) {
      state = inspectArtifact(origin, artifacts, index);
      states.set(origin, state);
      if (state.missing) {
        complete = false;
        missing.add(origin);
        if (requireAvailable) diagnostics.push(state.diagnostic);
      } else if (state.diagnostic !== null) {
        diagnostics.push(state.diagnostic);
      }
    }

    if (typeof record.span !== 'string' || state.bytes === null || state.diagnostic !== null) return;
    const match = record.span.match(SPAN);
    if (match === null || BigInt(match[1]) >= BigInt(match[2])) {
      diagnostics.push(diagnostic('AES_INVALID_SPAN', 'Span syntax or ordering is invalid.', index, 'span'));
      return;
    }
    const start = BigInt(match[1]);
    const end = BigInt(match[2]);
    const length = BigInt(state.bytes.length);
    if (end > length
      || !utf8Boundary(state.bytes, Number(start))
      || !utf8Boundary(state.bytes, Number(end))) {
      diagnostics.push(diagnostic(
        'AES_INVALID_SPAN',
        'Span exceeds the retained source artifact or splits a UTF-8 scalar.',
        index,
        'span',
      ));
    }
  });

  return {
    valid: diagnostics.length === 0,
    complete,
    verifiedOrigins: [...states]
      .filter(([, state]) => !state.missing && state.diagnostic === null)
      .map(([origin]) => origin),
    missingOrigins: [...missing],
    diagnostics,
  };
}

function inspectArtifact(origin, artifacts, record) {
  let candidate;
  try {
    candidate = typeof artifacts === 'function'
      ? artifacts(origin)
      : artifacts instanceof Map
        ? artifacts.get(origin)
        : plainRecord(artifacts)
          ? artifacts[origin]
          : undefined;
  } catch {
    candidate = undefined;
  }
  if (candidate === undefined || candidate === null) {
    return {
      bytes: null,
      missing: true,
      diagnostic: diagnostic('AES_SOURCE_REQUIRED', `Exact source artifact '${origin}' is unavailable.`, record, 'origin'),
    };
  }
  const bytes = exactBytes(candidate);
  if (bytes === null) {
    return {
      bytes: null,
      missing: false,
      diagnostic: diagnostic('AES_SOURCE_ARTIFACT_INVALID', 'Source artifact must be supplied as exact bytes.', record, 'origin'),
    };
  }
  const actual = `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
  if (actual !== origin) {
    return {
      bytes,
      missing: false,
      diagnostic: diagnostic('AES_ORIGIN_MISMATCH', 'Retained source bytes do not match the declared origin.', record, 'origin'),
    };
  }
  try {
    UTF8.decode(bytes);
  } catch {
    return {
      bytes,
      missing: false,
      diagnostic: diagnostic('AES_SOURCE_INVALID_UTF8', 'Retained source artifact is not valid UTF-8.', record, 'origin'),
    };
  }
  return { bytes, missing: false, diagnostic: null };
}

function exactBytes(value) {
  if (value instanceof Uint8Array) return Buffer.from(value.buffer, value.byteOffset, value.byteLength);
  if (value instanceof ArrayBuffer) return Buffer.from(value);
  return null;
}

function utf8Boundary(bytes, offset) {
  return offset === 0 || offset === bytes.length || (bytes[offset] & 0xc0) !== 0x80;
}

function plainRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null);
}

function diagnostic(code, message, record = null, field = null) {
  return { code, message, record, field };
}
