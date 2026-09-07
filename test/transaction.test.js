import assert from 'node:assert/strict';
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
  verifyAesTransactionDigest,
} from '../src/transaction.js';

function scalarBody(overrides = {}) {
  return {
    transaction: AES_TRANSACTION_CONTRACT,
    id: 'tx-1',
    intent: 'intent-1',
    attempt: 'attempt-1',
    events: 'aes.events.v0',
    profile: 'aes.partial.v0',
    projection: null,
    ordering: 'aes.order.exact.v0',
    application: { contract: AES_SCALAR_REPLACEMENT_APPLICATION },
    target: { contract: AES_ASP_TARGET, id: 'database-1', boundary: '$' },
    preconditions: [{ contract: AES_ASP_REVISION_PRECONDITION, scope: '$', revision: '7' }],
    preparation: { contract: AES_IDENTITY_PREPARATION },
    authorization: { contract: AES_HOST_AUTHORIZATION, context: 'host-auth-1' },
    limits: { contract: AES_LIMITS_CLAIM, id: 'altopelago.aeonic-limits.v1', version: '1.0.0' },
    assertions: { eventCount: '1', containers: [] },
    records: [{ path: '$.status', kind: 'StringLiteral', value: 'ready' }],
    integrity: { contract: AES_TRANSACTION_INTEGRITY, digest: 'sha256' },
    ...overrides,
  };
}

function evidence(body) {
  return {
    integrity: AES_TRANSACTION_INTEGRITY,
    digest: 'sha256',
    hash: computeAesTransactionDigest(body).digest,
    signatures: [],
  };
}

function support() {
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

test('validates the closed initial scalar transaction body', () => {
  assert.deepEqual(validateAesTransactionBody(scalarBody()), { valid: true, diagnostics: [] });
});

test('transaction digest binds target, authorization, limits, and exact payload', () => {
  const body = scalarBody();
  const digest = computeAesTransactionDigest(body);
  assert.match(digest.digest, /^[0-9a-f]{64}$/u);
  for (const changed of [
    scalarBody({ target: { ...body.target, id: 'database-2' } }),
    scalarBody({ authorization: { ...body.authorization, context: 'host-auth-2' } }),
    scalarBody({ limits: { ...body.limits, version: '1.0.1' } }),
    scalarBody({ records: [{ ...body.records[0], value: 'stopped' }] }),
  ]) {
    assert.notEqual(computeAesTransactionDigest(changed).digest, digest.digest);
  }
  assert.equal(verifyAesTransactionDigest(body, digest.digest).matches, true);
});

test('envelope evidence verifies without making unsupported contracts actionable', () => {
  const body = scalarBody();
  const envelope = { envelope: AES_TRANSACTION_ENVELOPE, body, evidence: evidence(body) };
  const unsupported = inspectAesTransactionEnvelope(envelope, { requireEvidence: true });
  assert.equal(unsupported.valid, true);
  assert.equal(unsupported.evidenceVerified, true);
  assert.equal(unsupported.supported, false);
  assert.equal(unsupported.readyForAuthorization, false);
  assert.equal(unsupported.actionable, false);

  const supported = inspectAesTransactionEnvelope(envelope, { requireEvidence: true, support: support() });
  assert.equal(supported.valid, true);
  assert.equal(supported.evidenceVerified, true);
  assert.equal(supported.supported, true);
  assert.equal(supported.readyForAuthorization, true);
  assert.equal(supported.actionable, false);
});

test('unsigned envelopes remain inspectable but fail evidence-required policy', () => {
  const envelope = { envelope: AES_TRANSACTION_ENVELOPE, body: scalarBody(), evidence: null };
  assert.equal(validateAesTransactionEnvelope(envelope).valid, true);
  const required = validateAesTransactionEnvelope(envelope, { requireEvidence: true });
  assert.equal(required.valid, false);
  assert.ok(required.diagnostics.some(({ code }) => code === 'AES_TRANSACTION_INTEGRITY_INVALID'));
});

test('scalar application rejects widening and assertion drift', () => {
  const base = scalarBody();
  for (const body of [
    scalarBody({ records: [{ ...base.records[0], datatype: 'string', generics: [], clarifiers: [] }] }),
    scalarBody({ records: [base.records[0], { path: '$.other', kind: 'StringLiteral', value: 'x' }], assertions: { eventCount: '2', containers: [] } }),
    scalarBody({ assertions: { eventCount: '2', containers: [] } }),
    scalarBody({ profile: 'aes.complete.v0' }),
  ]) {
    const result = validateAesTransactionBody(body);
    assert.equal(result.valid, false);
    assert.ok(result.diagnostics.some(({ code }) => code === 'AES_TRANSACTION_APPLICATION_INVALID'
      || code === 'AES_TRANSACTION_ASSERTION_FAILED'));
  }
});

test('unknown fields and repeated identities fail closed', () => {
  const extension = validateAesTransactionBody({ ...scalarBody(), 'x.example.note': 'bound' });
  assert.equal(extension.valid, false);
  assert.ok(extension.diagnostics.some(({ code }) => code === 'AES_TRANSACTION_EXTENSION_UNREGISTERED'));
  assert.equal(validateAesTransactionBody(
    { ...scalarBody(), 'x.example.note': 'bound' },
    { registeredFields: ['x.example.note'] },
  ).valid, true);
  assert.equal(validateAesTransactionBody(scalarBody({ attempt: 'tx-1' })).valid, false);
});

test('tampered evidence fails and signature input binds algorithm and key identity', () => {
  const body = scalarBody();
  const bad = evidence(body);
  bad.hash = `${bad.hash.slice(0, -1)}${bad.hash.endsWith('0') ? '1' : '0'}`;
  const envelope = { envelope: AES_TRANSACTION_ENVELOPE, body, evidence: bad };
  const result = validateAesTransactionEnvelope(envelope);
  assert.equal(result.valid, false);
  assert.ok(result.diagnostics.some(({ code }) => code === 'AES_TRANSACTION_INTEGRITY_MISMATCH'));

  const digest = computeAesTransactionDigest(body).digest;
  const alice = encodeAesTransactionSignatureInput({ digest, alg: 'ed25519', kid: 'alice' });
  const bob = encodeAesTransactionSignatureInput({ digest, alg: 'ed25519', kid: 'bob' });
  assert.notDeepEqual(alice.bytes, bob.bytes);
});
