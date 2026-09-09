import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';

import {
  AES_ASP_REVISION_PRECONDITION,
  AES_ASP_TARGET,
  AES_HOST_AUTHORIZATION,
  AES_IDENTITY_PREPARATION,
  AES_LIMITS_CLAIM,
  AES_SCALAR_REPLACEMENT_APPLICATION,
  AES_TRANSACTION_CONTRACT,
  AES_TRANSACTION_ENVELOPE,
  AES_TRANSACTION_INTEGRITY,
  computeAesTransactionDigest,
  encodeAesTransactionSignatureInput,
  inspectAesTransactionEnvelope,
  validateAesTransactionBody,
  validateAesTransactionEnvelope,
} from '../src/transaction.js';

const root = path.resolve(import.meta.dirname, '..');
const manifestPath = path.join(root, 'conformance/transactions/v1/aes-transaction-cts.v1.json');
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
const suites = await Promise.all(manifest.suites.map(async (entry) => JSON.parse(await readFile(
  path.join(path.dirname(manifestPath), entry.file),
  'utf8',
))));

function scalarBody() {
  return {
    transaction: AES_TRANSACTION_CONTRACT,
    id: 'tx-1', intent: 'intent-1', attempt: 'attempt-1',
    events: 'aes.events.v1', profile: 'aes.partial.v1', projection: null,
    ordering: 'aes.order.exact.v1',
    application: { contract: AES_SCALAR_REPLACEMENT_APPLICATION },
    target: { contract: AES_ASP_TARGET, id: 'database-1', boundary: '$' },
    preconditions: [{ contract: AES_ASP_REVISION_PRECONDITION, scope: '$', revision: '7' }],
    preparation: { contract: AES_IDENTITY_PREPARATION },
    authorization: { contract: AES_HOST_AUTHORIZATION, context: 'host-auth-1' },
    limits: { contract: AES_LIMITS_CLAIM, id: 'altopelago.aeonic-limits.v1', version: '1.0.0' },
    assertions: { eventCount: '1', containers: [] },
    records: [{ path: '$.status', kind: 'StringLiteral', value: 'ready' }],
    integrity: { contract: AES_TRANSACTION_INTEGRITY, digest: 'sha256' },
  };
}

function applyPatch(target, patch) {
  for (const [pathText, value] of Object.entries(patch ?? {})) {
    if (pathText.startsWith('x.')) {
      target[pathText] = structuredClone(value);
      continue;
    }
    const segments = pathText.split('.');
    let cursor = target;
    for (const segment of segments.slice(0, -1)) cursor = cursor[Number.isInteger(Number(segment)) ? Number(segment) : segment];
    cursor[segments.at(-1)] = structuredClone(value);
  }
  return target;
}

function bodyFor(input) {
  assert.equal(input.body_ref, 'scalar');
  return applyPatch(scalarBody(), input.patch);
}

function evidenceFor(input, body) {
  if (Object.hasOwn(input, 'evidence')) return input.evidence;
  if (input.evidence_ref === undefined) return null;
  const hash = computeAesTransactionDigest(body, { registeredFields: input.registered_fields ?? [] }).digest;
  return {
    integrity: AES_TRANSACTION_INTEGRITY,
    digest: 'sha256',
    hash: input.evidence_ref === 'tampered' ? `${hash.slice(0, -1)}${hash.endsWith('0') ? '1' : '0'}` : hash,
    signatures: [],
  };
}

function supportFor(input) {
  if (input.support_ref !== 'scalar') return {};
  return {
    applications: [AES_SCALAR_REPLACEMENT_APPLICATION],
    targets: [AES_ASP_TARGET],
    preconditions: [AES_ASP_REVISION_PRECONDITION],
    preparations: [AES_IDENTITY_PREPARATION],
    authorizations: [AES_HOST_AUTHORIZATION],
    limitSets: ['altopelago.aeonic-limits.v1@1.0.0'],
    integrityContracts: [AES_TRANSACTION_INTEGRITY],
  };
}

function codes(diagnostics) {
  return [...new Set(diagnostics.map(({ code }) => code))].sort();
}

test('AES transaction candidate manifest is internally consistent', () => {
  assert.equal(manifest.meta.status, 'draft');
  assert.equal(manifest.meta.transaction_contract, AES_TRANSACTION_CONTRACT);
  assert.equal(suites.length, manifest.suites.length);
  assert.equal(new Set(suites.flatMap((suite) => suite.tests.map(({ id }) => id))).size, 14);
});

for (const vector of suites.flatMap(({ tests }) => tests)) {
  test(vector.id, () => {
    if (vector.operation === 'signature-input') {
      const result = encodeAesTransactionSignatureInput(vector.input);
      assert.equal(result.bytes.toString('hex'), vector.expected.logical_hex);
      return;
    }
    const body = bodyFor(vector.input);
    const options = { registeredFields: vector.input.registered_fields ?? [] };
    if (vector.operation === 'digest') {
      const result = computeAesTransactionDigest(body, options);
      assert.equal(result.digest, vector.expected.digest);
      if (vector.expected.logical_hex !== undefined) assert.equal(result.bytes.toString('hex'), vector.expected.logical_hex);
      return;
    }
    if (vector.operation === 'validate-body') {
      const result = validateAesTransactionBody(body, options);
      assert.equal(result.valid, vector.expected.valid);
      assert.deepEqual(codes(result.diagnostics), [...vector.expected.diagnostic_codes].sort());
      return;
    }
    const envelope = { envelope: AES_TRANSACTION_ENVELOPE, body, evidence: evidenceFor(vector.input, body) };
    const envelopeOptions = { ...options, requireEvidence: vector.input.require_evidence === true };
    const result = vector.operation === 'inspect-envelope'
      ? inspectAesTransactionEnvelope(envelope, { ...envelopeOptions, support: supportFor(vector.input) })
      : validateAesTransactionEnvelope(envelope, envelopeOptions);
    for (const field of ['valid', 'supported', 'evidence_verified', 'ready_for_authorization', 'actionable']) {
      if (Object.hasOwn(vector.expected, field)) {
        const actualField = field.replace(/_([a-z])/gu, (_, letter) => letter.toUpperCase());
        assert.equal(result[actualField], vector.expected[field]);
      }
    }
    assert.deepEqual(codes(result.diagnostics), [...vector.expected.diagnostic_codes].sort());
  });
}

test('AES transaction candidate contains 14 vectors', () => {
  assert.equal(suites.flatMap(({ tests }) => tests).length, 14);
});
