import { Buffer } from 'node:buffer';
import { createHash, timingSafeEqual } from 'node:crypto';

import {
  AEON_DOCUMENT_PROJECTION,
  COMPLETE_AES_PROFILE,
  validateTelexRecords,
} from './telex.js';

export const AES_INTEGRITY_CONTRACT = 'aes.integrity.v1';
export const AES_EVENT_CONTRACT = 'aes.events.v1';
export const AES_CANONICAL_SEMANTIC_ORDER = 'aes.order.canonical-semantic.v1';
export const AES_EXACT_ORDER = 'aes.order.exact.v1';
export const AES_BODY_SCOPE = 'aes.scope.body.v1';
export const AES_DOCUMENT_SCOPE = 'aes.scope.document.v1';
export const AES_PROVENANCE_EXCLUDED = 'aes.provenance.excluded.v1';
export const AES_PROVENANCE_INCLUDED = 'aes.provenance.included.v1';
export const AES_DIGEST_SHA256 = 'sha256';
export const AES_SIGNATURE_CONTRACT = 'aes.signature.v1';

const INTEGRITY_DOMAIN = Buffer.from(`${AES_INTEGRITY_CONTRACT}\0`, 'utf8');
const SIGNATURE_DOMAIN = Buffer.from(`${AES_SIGNATURE_CONTRACT}\0`, 'utf8');
const ORDERING_POLICIES = new Set([AES_CANONICAL_SEMANTIC_ORDER, AES_EXACT_ORDER]);
const SCOPES = new Set([AES_BODY_SCOPE, AES_DOCUMENT_SCOPE]);
const PROVENANCE_POLICIES = new Set([AES_PROVENANCE_EXCLUDED, AES_PROVENANCE_INCLUDED]);
const LOWER_SHA256 = /^[0-9a-f]{64}$/u;

export class AesIntegrityError extends Error {
  constructor(code, message, details = {}) {
    super(message);
    this.name = 'AesIntegrityError';
    this.code = code;
    Object.assign(this, details);
  }
}

/**
 * Build the complete transport-neutral integrity input and deterministic bytes.
 * Ordering, scope, and provenance are intentionally required: AES never infers
 * whether a caller means canonical semantic state or exact supplied order.
 */
export function encodeAesIntegrity(records, options = {}) {
  if (!Array.isArray(records)) {
    throw integrityError('AES_INTEGRITY_CONTEXT_REQUIRED', 'AES integrity records must be an array.');
  }
  const profile = options.profile ?? COMPLETE_AES_PROFILE;
  const projection = options.projection ?? null;
  const ordering = requireIdentifier(options.ordering, 'ordering');
  const scope = requireIdentifier(options.scope, 'scope');
  const provenance = requireIdentifier(options.provenance, 'provenance');
  const digest = options.digest ?? AES_DIGEST_SHA256;

  if (!ORDERING_POLICIES.has(ordering)) {
    throw integrityError('AES_INTEGRITY_UNSUPPORTED_ORDERING', `Unsupported AES integrity ordering policy '${ordering}'.`);
  }
  if (!SCOPES.has(scope)) {
    throw integrityError('AES_INTEGRITY_UNSUPPORTED_SCOPE', `Unsupported AES integrity scope '${scope}'.`);
  }
  if (!PROVENANCE_POLICIES.has(provenance)) {
    throw integrityError('AES_INTEGRITY_UNSUPPORTED_PROVENANCE', `Unsupported AES integrity provenance policy '${provenance}'.`);
  }
  if (digest !== AES_DIGEST_SHA256) {
    throw integrityError('AES_INTEGRITY_UNSUPPORTED_DIGEST', `Unsupported AES digest identifier '${String(digest)}'.`);
  }
  if (scope === AES_DOCUMENT_SCOPE && projection !== AEON_DOCUMENT_PROJECTION) {
    throw integrityError(
      'AES_INTEGRITY_UNSUPPORTED_SCOPE',
      `Scope '${AES_DOCUMENT_SCOPE}' requires projection '${AEON_DOCUMENT_PROJECTION}'.`,
    );
  }

  const validation = validateTelexRecords(records, {
    ...options,
    profile,
    projection,
  });
  if (!validation.valid) {
    throw integrityError(
      'AES_INTEGRITY_INVALID_LOGICAL_VALUE',
      'AES integrity input is not valid under its selected event context.',
      { diagnostics: validation.diagnostics },
    );
  }

  let covered = records
    .filter((record) => scope === AES_DOCUMENT_SCOPE || hasOwn(record, 'path'))
    .map((record) => projectRecord(record, provenance));

  if (ordering === AES_CANONICAL_SEMANTIC_ORDER) {
    assertUniqueAddresses(covered);
    covered = [...covered].sort(compareRecordsByAddress);
  }

  const input = {
    integrity: AES_INTEGRITY_CONTRACT,
    events: AES_EVENT_CONTRACT,
    profile,
    projection,
    ordering,
    scope,
    provenance,
    digest,
    records: covered,
  };
  const bytes = Buffer.concat([INTEGRITY_DOMAIN, encodeLogicalValue(input)]);
  return { input, bytes };
}

export function computeAesIntegrityDigest(records, options = {}) {
  const encoded = encodeAesIntegrity(records, options);
  return {
    ...encoded,
    algorithm: AES_DIGEST_SHA256,
    digest: createHash('sha256').update(encoded.bytes).digest('hex'),
  };
}

export function verifyAesIntegrityDigest(records, expectedDigest, options = {}) {
  if (typeof expectedDigest !== 'string' || !LOWER_SHA256.test(expectedDigest)) {
    throw integrityError(
      'AES_INTEGRITY_DIGEST_MISMATCH',
      'Expected AES integrity digest must be 64 lowercase hexadecimal digits.',
    );
  }
  const computed = computeAesIntegrityDigest(records, options);
  const matches = timingSafeEqual(Buffer.from(computed.digest, 'hex'), Buffer.from(expectedDigest, 'hex'));
  return { ...computed, matches };
}

/** Build the domain-separated bytes authenticated by an AES signature entry. */
export function encodeAesSignatureInput({ digest, alg, kid } = {}) {
  if (typeof digest !== 'string' || !LOWER_SHA256.test(digest)
    || typeof alg !== 'string' || alg.length === 0
    || typeof kid !== 'string' || kid.length === 0
    || !hasOnlyUnicodeScalars(alg) || !hasOnlyUnicodeScalars(kid)) {
    throw integrityError(
      'AES_SIGNATURE_CONTEXT_INVALID',
      'AES signature context requires a canonical SHA-256 digest and non-empty scalar-value alg and kid strings.',
    );
  }
  const context = {
    signature: AES_SIGNATURE_CONTRACT,
    integrity: AES_INTEGRITY_CONTRACT,
    digest: AES_DIGEST_SHA256,
    hash: digest,
    alg,
    kid,
  };
  return { context, bytes: Buffer.concat([SIGNATURE_DOMAIN, encodeLogicalValue(context)]) };
}

/** Encode the small null/string/list/map value domain owned by aes.integrity.v1. */
export function encodeAesIntegrityValue(value) {
  return encodeLogicalValue(value);
}

function projectRecord(source, provenance) {
  const entries = source instanceof Map ? [...source] : Object.entries(source);
  const record = {};
  for (const [key, value] of entries) {
    if (provenance === AES_PROVENANCE_EXCLUDED && (key === 'origin' || key === 'span')) continue;
    record[key] = cloneLogicalValue(value);
  }
  return record;
}

function cloneLogicalValue(value) {
  if (value === null || typeof value === 'string') return value;
  if (Array.isArray(value)) return value.map(cloneLogicalValue);
  if (value instanceof Map) {
    const result = new Map();
    for (const [key, child] of value) {
      if (typeof key !== 'string' || result.has(key)) {
        throw integrityError('AES_INTEGRITY_INVALID_LOGICAL_VALUE', 'Integrity map keys must be unique strings.');
      }
      result.set(key, cloneLogicalValue(child));
    }
    return result;
  }
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, child]) => [key, cloneLogicalValue(child)]));
  }
  throw integrityError('AES_INTEGRITY_INVALID_LOGICAL_VALUE', `Unsupported AES integrity logical value '${typeof value}'.`);
}

function assertUniqueAddresses(records) {
  const seen = new Set();
  for (const record of records) {
    const field = hasOwn(record, 'header') ? 'header' : 'path';
    const key = `${field}\0${record[field]}`;
    if (seen.has(key)) {
      throw integrityError(
        'AES_INTEGRITY_AMBIGUOUS_CANONICAL_ORDER',
        `Canonical-semantic ordering requires unique ${field} address '${record[field]}'.`,
        { field, address: record[field] },
      );
    }
    seen.add(key);
  }
}

function compareRecordsByAddress(left, right) {
  const leftHeader = hasOwn(left, 'header');
  const rightHeader = hasOwn(right, 'header');
  if (leftHeader !== rightHeader) return leftHeader ? -1 : 1;
  const field = leftHeader ? 'header' : 'path';
  return Buffer.compare(Buffer.from(left[field], 'utf8'), Buffer.from(right[field], 'utf8'));
}

function encodeLogicalValue(value) {
  if (value === null) return Buffer.from('n', 'ascii');
  if (typeof value === 'string') {
    if (!hasOnlyUnicodeScalars(value)) {
      throw integrityError('AES_INTEGRITY_INVALID_LOGICAL_VALUE', 'Integrity strings must contain only Unicode scalar values.');
    }
    const payload = Buffer.from(value, 'utf8');
    return Buffer.concat([Buffer.from(`s${payload.length}:`, 'ascii'), payload]);
  }
  if (Array.isArray(value)) {
    return Buffer.concat([
      Buffer.from(`l${value.length}:`, 'ascii'),
      ...value.map(encodeLogicalValue),
    ]);
  }
  if (value instanceof Map || (value !== null && typeof value === 'object')) {
    const entries = value instanceof Map ? [...value] : Object.entries(value);
    const members = new Map();
    for (const [key, child] of entries) {
      if (typeof key !== 'string' || members.has(key) || !hasOnlyUnicodeScalars(key)) {
        throw integrityError('AES_INTEGRITY_INVALID_LOGICAL_VALUE', 'Integrity map keys must be unique Unicode-scalar strings.');
      }
      members.set(key, child);
    }
    const keys = [...members.keys()].sort(compareStringsByUtf8);
    const chunks = [Buffer.from(`m${keys.length}:`, 'ascii')];
    for (const key of keys) {
      chunks.push(encodeLogicalValue(key), encodeLogicalValue(members.get(key)));
    }
    return Buffer.concat(chunks);
  }
  throw integrityError('AES_INTEGRITY_INVALID_LOGICAL_VALUE', `Unsupported AES integrity logical value '${typeof value}'.`);
}

function compareStringsByUtf8(left, right) {
  return Buffer.compare(Buffer.from(left, 'utf8'), Buffer.from(right, 'utf8'));
}

function requireIdentifier(value, label) {
  if (typeof value !== 'string' || value.length === 0 || !hasOnlyUnicodeScalars(value)) {
    throw integrityError('AES_INTEGRITY_CONTEXT_REQUIRED', `AES integrity ${label} must be an explicit non-empty identifier.`);
  }
  return value;
}

function hasOnlyUnicodeScalars(value) {
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) return false;
      index += 1;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      return false;
    }
  }
  return true;
}

function hasOwn(value, key) {
  return value instanceof Map
    ? value.has(key)
    : value !== null && typeof value === 'object' && Object.hasOwn(value, key);
}

function integrityError(code, message, details = {}) {
  return new AesIntegrityError(code, message, details);
}
