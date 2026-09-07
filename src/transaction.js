import { Buffer } from 'node:buffer';
import { createHash, timingSafeEqual } from 'node:crypto';

import { encodeAesIntegrityValue } from './integrity.js';
import { PARTIAL_AES_PROFILE, validateTelexRecords } from './telex.js';

export const AES_TRANSACTION_CONTRACT = 'aes.transaction.v0';
export const AES_TRANSACTION_ENVELOPE = 'aes.transaction.envelope.v0';
export const AES_TRANSACTION_INTEGRITY = 'aes.transaction.integrity.v0';
export const AES_TRANSACTION_SIGNATURE = 'aes.transaction.signature.v0';
export const AES_SCALAR_REPLACEMENT_APPLICATION = 'aes.application.asp.scalar-replacement.v0';
export const AES_ASP_TARGET = 'aes.target.asp.v0';
export const AES_ASP_REVISION_PRECONDITION = 'aes.precondition.asp-revision.v0';
export const AES_IDENTITY_PREPARATION = 'aes.preparation.identity.v0';
export const AES_HOST_AUTHORIZATION = 'aes.authorization.host-context.v0';
export const AES_LIMITS_CLAIM = 'aes.limits.claim.v0';

const TRANSACTION_DOMAIN = Buffer.from(`${AES_TRANSACTION_INTEGRITY}\0`, 'utf8');
const SIGNATURE_DOMAIN = Buffer.from(`${AES_TRANSACTION_SIGNATURE}\0`, 'utf8');
const LOWER_SHA256 = /^[0-9a-f]{64}$/u;
const UNSIGNED_DECIMAL = /^(?:0|[1-9][0-9]*)$/u;
const EXTENSION_FIELD = /^x\.[a-z][a-z0-9-]*(?:\.[a-z][a-z0-9-]*)+$/u;
const BODY_FIELDS = new Set([
  'transaction', 'id', 'intent', 'attempt', 'events', 'profile', 'projection',
  'ordering', 'application', 'target', 'preconditions', 'preparation',
  'authorization', 'limits', 'assertions', 'records', 'integrity',
]);
const SCALAR_KINDS = new Set([
  'StringLiteral', 'NumberLiteral', 'InfinityLiteral', 'NaNLiteral',
  'BooleanLiteral', 'ToggleLiteral', 'NullLiteral', 'HexLiteral',
  'RadixLiteral', 'EncodingLiteral', 'SeparatorLiteral', 'SansaAddressLiteral',
  'DateLiteral', 'TimeLiteral', 'DateTimeLiteral', 'WTCDateTimeLiteral',
]);

export class AesTransactionError extends Error {
  constructor(code, message, details = {}) {
    super(message);
    this.name = 'AesTransactionError';
    this.code = code;
    Object.assign(this, details);
  }
}

/** Validate the transport-neutral body without granting application authority. */
export function validateAesTransactionBody(body, options = {}) {
  const diagnostics = [];
  if (!plainRecord(body)) {
    return invalid('AES_TRANSACTION_BODY_INVALID', 'Transaction body must be a map.');
  }

  const registeredFields = new Set(options.registeredFields ?? []);
  for (const field of Object.keys(body)) {
    if (BODY_FIELDS.has(field)) continue;
    if (!EXTENSION_FIELD.test(field) || !registeredFields.has(field)) {
      diagnostics.push(diagnostic(
        field.startsWith('x.') ? 'AES_TRANSACTION_EXTENSION_UNREGISTERED' : 'AES_TRANSACTION_BODY_INVALID',
        `Transaction body field '${field}' is not registered.`,
        field,
      ));
    }
  }

  exact(body.transaction, AES_TRANSACTION_CONTRACT, 'transaction', diagnostics);
  exact(body.events, 'aes.events.v0', 'events', diagnostics);
  exact(body.ordering, 'aes.order.exact.v0', 'ordering', diagnostics);
  for (const field of ['id', 'intent', 'attempt']) identifier(body[field], field, diagnostics);
  if ([body.id, body.intent, body.attempt].every((value) => typeof value === 'string')
    && new Set([body.id, body.intent, body.attempt]).size !== 3) {
    diagnostics.push(diagnostic('AES_TRANSACTION_BODY_INVALID', 'Transaction, intent, and attempt identities must be distinct.', 'id'));
  }
  identifier(body.profile, 'profile', diagnostics);
  if (body.projection !== null) identifier(body.projection, 'projection', diagnostics);

  contractMap(body.application, 'application', diagnostics, ['contract']);
  validateTarget(body.target, diagnostics);
  validatePreconditions(body.preconditions, diagnostics);
  contractMap(body.preparation, 'preparation', diagnostics, ['contract']);
  validateAuthorization(body.authorization, diagnostics);
  validateLimits(body.limits, diagnostics);
  validateAssertions(body.assertions, body.records, diagnostics);
  validateIntegrityPolicy(body.integrity, diagnostics);

  if (!Array.isArray(body.records)) {
    diagnostics.push(diagnostic('AES_TRANSACTION_BODY_INVALID', 'records must be an ordered list.', 'records'));
  } else if (typeof body.profile === 'string' && (body.projection === null || typeof body.projection === 'string')) {
    const eventValidation = validateTelexRecords(body.records, {
      profile: body.profile,
      projection: body.projection,
      registeredFields: options.registeredEventFields ?? [],
      limits: options.limits,
    });
    if (!eventValidation.valid) {
      diagnostics.push(...eventValidation.diagnostics.map((item) => diagnostic(
        'AES_TRANSACTION_EVENT_INVALID',
        `${item.code}: ${item.message}`,
        item.path ?? 'records',
      )));
    }
  }

  if (plainRecord(body.application)
    && body.application.contract === AES_SCALAR_REPLACEMENT_APPLICATION) {
    validateScalarReplacement(body, diagnostics);
  }

  try {
    encodeAesIntegrityValue(body);
  } catch (cause) {
    diagnostics.push(diagnostic(
      'AES_TRANSACTION_BODY_INVALID',
      cause instanceof Error ? cause.message : 'Transaction body contains an unsupported logical value.',
    ));
  }
  return { valid: diagnostics.length === 0, diagnostics };
}

/** Check consumer capability separately from structural transaction validity. */
export function checkAesTransactionSupport(body, options = {}) {
  if (!plainRecord(body)) return invalid('AES_TRANSACTION_BODY_INVALID', 'Transaction body must be a map.');
  const requirements = [
    ['application', body.application?.contract, options.applications],
    ['target', body.target?.contract, options.targets],
    ['preparation', body.preparation?.contract, options.preparations],
    ['authorization', body.authorization?.contract, options.authorizations],
    ['limits', body.limits?.id && body.limits?.version ? `${body.limits.id}@${body.limits.version}` : undefined, options.limitSets],
    ['integrity', body.integrity?.contract, options.integrityContracts],
  ];
  if (Array.isArray(body.preconditions)) {
    for (const entry of body.preconditions) requirements.push(['precondition', entry?.contract, options.preconditions]);
  }
  const unsupported = requirements
    .filter(([, value, supported]) => typeof value !== 'string' || !(supported instanceof Set ? supported : new Set(supported ?? [])).has(value))
    .map(([component, value]) => ({ component, contract: value ?? null }));
  return {
    supported: unsupported.length === 0,
    diagnostics: unsupported.map(({ component, contract }) => diagnostic(
      'AES_TRANSACTION_UNSUPPORTED_CONTRACT',
      `Unsupported ${component} contract '${String(contract)}'.`,
      component,
    )),
  };
}

export function computeAesTransactionDigest(body, options = {}) {
  const validation = validateAesTransactionBody(body, options);
  if (!validation.valid) {
    throw transactionError('AES_TRANSACTION_BODY_INVALID', 'Cannot digest an invalid AES transaction body.', {
      diagnostics: validation.diagnostics,
    });
  }
  const bytes = Buffer.concat([TRANSACTION_DOMAIN, encodeAesIntegrityValue(body)]);
  return {
    bytes,
    algorithm: 'sha256',
    digest: createHash('sha256').update(bytes).digest('hex'),
  };
}

export function verifyAesTransactionDigest(body, expected, options = {}) {
  if (typeof expected !== 'string' || !LOWER_SHA256.test(expected)) {
    throw transactionError('AES_TRANSACTION_INTEGRITY_INVALID', 'Transaction digest must be 64 lowercase hexadecimal digits.');
  }
  const computed = computeAesTransactionDigest(body, options);
  return {
    ...computed,
    matches: timingSafeEqual(Buffer.from(computed.digest, 'hex'), Buffer.from(expected, 'hex')),
  };
}

export function encodeAesTransactionSignatureInput({ digest, alg, kid } = {}) {
  if (typeof digest !== 'string' || !LOWER_SHA256.test(digest)
    || !nonEmptyScalarString(alg) || !nonEmptyScalarString(kid)) {
    throw transactionError('AES_TRANSACTION_SIGNATURE_INVALID', 'Transaction signature context is invalid.');
  }
  const context = {
    signature: AES_TRANSACTION_SIGNATURE,
    integrity: AES_TRANSACTION_INTEGRITY,
    digest: 'sha256',
    hash: digest,
    alg,
    kid,
  };
  return { context, bytes: Buffer.concat([SIGNATURE_DOMAIN, encodeAesIntegrityValue(context)]) };
}

export function validateAesTransactionEnvelope(envelope, options = {}) {
  const diagnostics = [];
  if (!plainRecord(envelope)) return invalid('AES_TRANSACTION_ENVELOPE_INVALID', 'Transaction envelope must be a map.');
  unknown(envelope, ['envelope', 'body', 'evidence'], 'envelope', diagnostics);
  exact(envelope.envelope, AES_TRANSACTION_ENVELOPE, 'envelope', diagnostics, 'AES_TRANSACTION_ENVELOPE_INVALID');
  const bodyValidation = validateAesTransactionBody(envelope.body, options);
  diagnostics.push(...bodyValidation.diagnostics);
  let evidenceVerified = false;
  if (envelope.evidence === null) {
    if (options.requireEvidence === true) {
      diagnostics.push(diagnostic('AES_TRANSACTION_INTEGRITY_INVALID', 'Trusted policy requires transaction integrity evidence.', 'evidence'));
    }
  } else if (!plainRecord(envelope.evidence)) {
    diagnostics.push(diagnostic('AES_TRANSACTION_INTEGRITY_INVALID', 'Evidence must be null or a map.', 'evidence'));
  } else {
    validateEvidence(envelope.evidence, diagnostics);
    if (bodyValidation.valid && typeof envelope.evidence.hash === 'string' && LOWER_SHA256.test(envelope.evidence.hash)) {
      evidenceVerified = verifyAesTransactionDigest(envelope.body, envelope.evidence.hash, options).matches;
      if (!evidenceVerified) diagnostics.push(diagnostic('AES_TRANSACTION_INTEGRITY_MISMATCH', 'Transaction integrity digest does not match the body.', 'evidence.hash'));
    }
  }
  return { valid: diagnostics.length === 0, evidenceVerified, diagnostics };
}

export function inspectAesTransactionEnvelope(envelope, options = {}) {
  const validation = validateAesTransactionEnvelope(envelope, options);
  const support = checkAesTransactionSupport(envelope?.body, options.support ?? {});
  const evidenceSatisfied = options.requireEvidence !== true || validation.evidenceVerified;
  const readyForAuthorization = validation.valid && support.supported && evidenceSatisfied;
  return {
    valid: validation.valid,
    supported: support.supported,
    evidenceVerified: validation.evidenceVerified,
    readyForAuthorization,
    actionable: false,
    diagnostics: [...validation.diagnostics, ...support.diagnostics],
  };
}

function validateTarget(value, diagnostics) {
  contractMap(value, 'target', diagnostics, ['contract', 'id', 'boundary']);
  if (!plainRecord(value)) return;
  identifier(value.id, 'target.id', diagnostics);
  identifier(value.boundary, 'target.boundary', diagnostics);
}

function validatePreconditions(value, diagnostics) {
  if (!Array.isArray(value)) {
    diagnostics.push(diagnostic('AES_TRANSACTION_BODY_INVALID', 'preconditions must be an ordered list.', 'preconditions'));
    return;
  }
  value.forEach((entry, index) => {
    contractMap(entry, `preconditions[${index}]`, diagnostics, ['contract', 'scope', 'revision']);
    if (!plainRecord(entry)) return;
    identifier(entry.scope, `preconditions[${index}].scope`, diagnostics);
    unsigned(entry.revision, `preconditions[${index}].revision`, diagnostics);
  });
}

function validateAuthorization(value, diagnostics) {
  contractMap(value, 'authorization', diagnostics, ['contract', 'context']);
  if (plainRecord(value)) identifier(value.context, 'authorization.context', diagnostics);
}

function validateLimits(value, diagnostics) {
  contractMap(value, 'limits', diagnostics, ['contract', 'id', 'version']);
  if (!plainRecord(value)) return;
  exact(value.contract, AES_LIMITS_CLAIM, 'limits.contract', diagnostics);
  identifier(value.id, 'limits.id', diagnostics);
  identifier(value.version, 'limits.version', diagnostics);
}

function validateAssertions(value, records, diagnostics) {
  if (!plainRecord(value)) {
    diagnostics.push(diagnostic('AES_TRANSACTION_BODY_INVALID', 'assertions must be a map.', 'assertions'));
    return;
  }
  unknown(value, ['eventCount', 'containers'], 'assertions', diagnostics);
  unsigned(value.eventCount, 'assertions.eventCount', diagnostics);
  if (Array.isArray(records) && UNSIGNED_DECIMAL.test(value.eventCount ?? '')
    && BigInt(value.eventCount) !== BigInt(records.length)) {
    diagnostics.push(diagnostic('AES_TRANSACTION_ASSERTION_FAILED', 'eventCount does not match records length.', 'assertions.eventCount'));
  }
  if (!Array.isArray(value.containers)) {
    diagnostics.push(diagnostic('AES_TRANSACTION_BODY_INVALID', 'assertions.containers must be an ordered list.', 'assertions.containers'));
    return;
  }
  value.containers.forEach((entry, index) => {
    const path = `assertions.containers[${index}]`;
    if (!plainRecord(entry)) {
      diagnostics.push(diagnostic('AES_TRANSACTION_BODY_INVALID', 'Container assertion must be a map.', path));
      return;
    }
    unknown(entry, ['path', 'payloadDirectItemCount', 'resultDirectItemCount'], path, diagnostics);
    identifier(entry.path, `${path}.path`, diagnostics);
    unsigned(entry.payloadDirectItemCount, `${path}.payloadDirectItemCount`, diagnostics);
    unsigned(entry.resultDirectItemCount, `${path}.resultDirectItemCount`, diagnostics);
  });
}

function validateIntegrityPolicy(value, diagnostics) {
  contractMap(value, 'integrity', diagnostics, ['contract', 'digest']);
  if (!plainRecord(value)) return;
  exact(value.contract, AES_TRANSACTION_INTEGRITY, 'integrity.contract', diagnostics);
  exact(value.digest, 'sha256', 'integrity.digest', diagnostics);
}

function validateScalarReplacement(body, diagnostics) {
  if (body.profile !== PARTIAL_AES_PROFILE || body.projection !== null) {
    diagnostics.push(diagnostic('AES_TRANSACTION_APPLICATION_INVALID', 'Scalar replacement requires aes.partial.v0 body-only event context.', 'profile'));
  }
  if (body.target?.contract !== AES_ASP_TARGET || body.target?.boundary !== '$') {
    diagnostics.push(diagnostic('AES_TRANSACTION_APPLICATION_INVALID', 'Scalar replacement requires the root-boundary ASP target contract.', 'target'));
  }
  if (!Array.isArray(body.preconditions) || body.preconditions.length !== 1
    || body.preconditions[0]?.contract !== AES_ASP_REVISION_PRECONDITION
    || body.preconditions[0]?.scope !== '$') {
    diagnostics.push(diagnostic('AES_TRANSACTION_APPLICATION_INVALID', 'Scalar replacement requires one root ASP revision precondition.', 'preconditions'));
  }
  if (body.preparation?.contract !== AES_IDENTITY_PREPARATION
    || body.authorization?.contract !== AES_HOST_AUTHORIZATION) {
    diagnostics.push(diagnostic('AES_TRANSACTION_APPLICATION_INVALID', 'Scalar replacement requires identity preparation and host authorization context.', 'application'));
  }
  if (body.assertions?.eventCount !== '1' || body.assertions?.containers?.length !== 0
    || !Array.isArray(body.records) || body.records.length !== 1) {
    diagnostics.push(diagnostic('AES_TRANSACTION_APPLICATION_INVALID', 'Scalar replacement carries exactly one record and no container assertions.', 'records'));
    return;
  }
  const record = body.records[0];
  if (!plainRecord(record) || Object.keys(record).some((field) => !['path', 'kind', 'value'].includes(field))
    || !SCALAR_KINDS.has(record.kind) || typeof record.value !== 'string') {
    diagnostics.push(diagnostic('AES_TRANSACTION_APPLICATION_INVALID', 'Scalar replacement record must contain only path, admitted scalar kind, and string value.', 'records[0]'));
  }
}

function validateEvidence(value, diagnostics) {
  unknown(value, ['integrity', 'digest', 'hash', 'signatures'], 'evidence', diagnostics, 'AES_TRANSACTION_INTEGRITY_INVALID');
  exact(value.integrity, AES_TRANSACTION_INTEGRITY, 'evidence.integrity', diagnostics, 'AES_TRANSACTION_INTEGRITY_INVALID');
  exact(value.digest, 'sha256', 'evidence.digest', diagnostics, 'AES_TRANSACTION_INTEGRITY_INVALID');
  if (typeof value.hash !== 'string' || !LOWER_SHA256.test(value.hash)) {
    diagnostics.push(diagnostic('AES_TRANSACTION_INTEGRITY_INVALID', 'Evidence hash must be canonical SHA-256 hexadecimal.', 'evidence.hash'));
  }
  if (!Array.isArray(value.signatures)) {
    diagnostics.push(diagnostic('AES_TRANSACTION_INTEGRITY_INVALID', 'Evidence signatures must be an ordered list.', 'evidence.signatures'));
    return;
  }
  value.signatures.forEach((entry, index) => {
    const path = `evidence.signatures[${index}]`;
    if (!plainRecord(entry)) {
      diagnostics.push(diagnostic('AES_TRANSACTION_SIGNATURE_INVALID', 'Signature entry must be a map.', path));
      return;
    }
    unknown(entry, ['signature', 'alg', 'kid', 'sig'], path, diagnostics, 'AES_TRANSACTION_SIGNATURE_INVALID');
    exact(entry.signature, AES_TRANSACTION_SIGNATURE, `${path}.signature`, diagnostics, 'AES_TRANSACTION_SIGNATURE_INVALID');
    if (!nonEmptyScalarString(entry.alg) || !nonEmptyScalarString(entry.kid) || !nonEmptyScalarString(entry.sig)) {
      diagnostics.push(diagnostic('AES_TRANSACTION_SIGNATURE_INVALID', 'Signature alg, kid, and sig must be non-empty strings.', path));
    }
  });
}

function contractMap(value, path, diagnostics, fields) {
  if (!plainRecord(value)) {
    diagnostics.push(diagnostic('AES_TRANSACTION_BODY_INVALID', `${path} must be a map.`, path));
    return;
  }
  unknown(value, fields, path, diagnostics);
  identifier(value.contract, `${path}.contract`, diagnostics);
}

function unknown(value, allowed, path, diagnostics, code = 'AES_TRANSACTION_BODY_INVALID') {
  if (!plainRecord(value)) return;
  for (const field of Object.keys(value)) {
    if (!allowed.includes(field)) diagnostics.push(diagnostic(code, `${path} field '${field}' is not allowed.`, `${path}.${field}`));
  }
}

function exact(value, expected, path, diagnostics, code = 'AES_TRANSACTION_BODY_INVALID') {
  if (value !== expected) diagnostics.push(diagnostic(code, `${path} must be '${expected}'.`, path));
}

function identifier(value, path, diagnostics) {
  if (!nonEmptyScalarString(value) || [...value].length > 256) {
    diagnostics.push(diagnostic('AES_TRANSACTION_BODY_INVALID', `${path} must contain 1 to 256 Unicode scalar values.`, path));
  }
}

function unsigned(value, path, diagnostics) {
  if (typeof value !== 'string' || !UNSIGNED_DECIMAL.test(value)) {
    diagnostics.push(diagnostic('AES_TRANSACTION_BODY_INVALID', `${path} must be a canonical unsigned decimal string.`, path));
  }
}

function nonEmptyScalarString(value) {
  if (typeof value !== 'string' || value.length === 0) return false;
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) return false;
      index += 1;
    } else if (code >= 0xdc00 && code <= 0xdfff) return false;
  }
  return true;
}

function plainRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    && (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null);
}

function diagnostic(code, message, path = null) {
  return { code, message, path };
}

function invalid(code, message) {
  return { valid: false, diagnostics: [diagnostic(code, message)] };
}

function transactionError(code, message, details = {}) {
  return new AesTransactionError(code, message, details);
}
