import assert from 'node:assert/strict';
import test from 'node:test';

import {
  AES_BODY_SCOPE,
  AES_CANONICAL_SEMANTIC_ORDER,
  AES_DOCUMENT_SCOPE,
  AES_EXACT_ORDER,
  AES_PROVENANCE_EXCLUDED,
  AES_PROVENANCE_INCLUDED,
  computeAesIntegrityDigest,
  encodeAesIntegrity,
  encodeAesIntegrityValue,
  encodeAesSignatureInput,
  verifyAesIntegrityDigest,
} from '../src/integrity.js';

const semanticCanonical = {
  ordering: AES_CANONICAL_SEMANTIC_ORDER,
  scope: AES_BODY_SCOPE,
  provenance: AES_PROVENANCE_EXCLUDED,
};

test('encodes null, strings, lists, and maps with byte lengths and UTF-8 key order', () => {
  assert.equal(encodeAesIntegrityValue(null).toString(), 'n');
  assert.equal(encodeAesIntegrityValue('🌊').toString(), 's4:🌊');
  assert.equal(encodeAesIntegrityValue(['a', '']).toString(), 'l2:s1:as0:');
  assert.equal(encodeAesIntegrityValue({ z: '2', a: '1' }).toString(), 'm2:s1:as1:1s1:zs1:2');
  assert.deepEqual(
    encodeAesIntegrityValue(new Map([['z', '2'], ['a', '1']])),
    encodeAesIntegrityValue({ a: '1', z: '2' }),
  );
});

test('canonical-semantic order is independent of supplied unique record order', () => {
  const records = [
    { path: '$.b', kind: 'NumberLiteral', value: '2' },
    { path: '$.a', kind: 'NumberLiteral', value: '1' },
  ];
  const first = computeAesIntegrityDigest(records, semanticCanonical);
  const second = computeAesIntegrityDigest([...records].reverse(), semanticCanonical);
  assert.equal(first.digest, second.digest);
  assert.deepEqual(first.input.records.map(({ path }) => path), ['$.a', '$.b']);
});

test('exact order distinguishes reordered records', () => {
  const records = [
    { path: '$.a', kind: 'NumberLiteral', value: '1' },
    { path: '$.b', kind: 'NumberLiteral', value: '2' },
  ];
  const options = { ...semanticCanonical, ordering: AES_EXACT_ORDER };
  assert.notEqual(
    computeAesIntegrityDigest(records, options).digest,
    computeAesIntegrityDigest([...records].reverse(), options).digest,
  );
});

test('canonical-semantic order rejects duplicate partial-stream addresses', () => {
  assert.throws(
    () => encodeAesIntegrity([
      { path: '$.a', kind: 'NumberLiteral', value: '1' },
      { path: '$.a', kind: 'NumberLiteral', value: '2' },
    ], { ...semanticCanonical, profile: 'aes.partial.v0' }),
    { code: 'AES_INTEGRITY_AMBIGUOUS_CANONICAL_ORDER' },
  );
});

test('exact order retains duplicate partial-stream occurrences', () => {
  const result = encodeAesIntegrity([
    { path: '$.a', kind: 'NumberLiteral', value: '1' },
    { path: '$.a', kind: 'NumberLiteral', value: '2' },
  ], { ...semanticCanonical, profile: 'aes.partial.v0', ordering: AES_EXACT_ORDER });
  assert.deepEqual(result.input.records.map(({ value }) => value), ['1', '2']);
});

test('semantic provenance policy excludes source coordinates', () => {
  const origin = `sha256:${'a'.repeat(64)}`;
  const plain = [{ path: '$.a', kind: 'StringLiteral', value: 'x' }];
  const sourced = [{ ...plain[0], origin, span: '0:1' }];
  assert.equal(
    computeAesIntegrityDigest(plain, semanticCanonical).digest,
    computeAesIntegrityDigest(sourced, semanticCanonical).digest,
  );
  assert.notEqual(
    computeAesIntegrityDigest(plain, { ...semanticCanonical, provenance: AES_PROVENANCE_INCLUDED }).digest,
    computeAesIntegrityDigest(sourced, { ...semanticCanonical, provenance: AES_PROVENANCE_INCLUDED }).digest,
  );
});

test('body and document scope remain distinct and document scope requires its projection', () => {
  const records = [
    { header: '$.["aeon:mode"]', kind: 'StringLiteral', value: 'strict' },
    { path: '$.a', kind: 'StringLiteral', value: 'x' },
  ];
  const body = computeAesIntegrityDigest(records, {
    ...semanticCanonical,
    projection: 'aeon.document.v0',
  });
  const document = computeAesIntegrityDigest(records, {
    ...semanticCanonical,
    projection: 'aeon.document.v0',
    scope: AES_DOCUMENT_SCOPE,
  });
  assert.equal(body.input.records.length, 1);
  assert.equal(document.input.records.length, 2);
  assert.notEqual(body.digest, document.digest);
  assert.throws(
    () => encodeAesIntegrity([{ path: '$.a', kind: 'StringLiteral', value: 'x' }], {
      ...semanticCanonical,
      scope: AES_DOCUMENT_SCOPE,
    }),
    { code: 'AES_INTEGRITY_UNSUPPORTED_SCOPE' },
  );
});

test('expanded datatype structure and extensions are covered structurally', () => {
  const base = {
    path: '$.a',
    kind: 'SeparatorLiteral',
    datatype: 'csv',
    generics: [],
    clarifiers: [{ kind: 'StringLiteral', value: '.' }],
    value: 'one.two',
    'x.example.note': 'kept',
  };
  const first = computeAesIntegrityDigest([base], {
    ...semanticCanonical,
    registeredFields: ['x.example.note'],
  });
  const second = computeAesIntegrityDigest([{ ...base, clarifiers: [{ kind: 'StringLiteral', value: ',' }] }], {
    ...semanticCanonical,
    registeredFields: ['x.example.note'],
  });
  assert.notEqual(first.digest, second.digest);
});

test('signature input binds digest, algorithm, and key identity', () => {
  const digest = computeAesIntegrityDigest([], semanticCanonical).digest;
  const first = encodeAesSignatureInput({ digest, alg: 'ed25519', kid: 'alice' });
  const second = encodeAesSignatureInput({ digest, alg: 'ed25519', kid: 'bob' });
  assert.notDeepEqual(first.bytes, second.bytes);
  assert.match(first.bytes.toString('hex'), /^6165732e7369676e61747572652e763000/u);
});

test('digest verification rejects non-canonical text and compares canonical bytes', () => {
  const digest = computeAesIntegrityDigest([], semanticCanonical).digest;
  assert.equal(verifyAesIntegrityDigest([], digest, semanticCanonical).matches, true);
  assert.throws(
    () => verifyAesIntegrityDigest([], digest.toUpperCase(), semanticCanonical),
    { code: 'AES_INTEGRITY_DIGEST_MISMATCH' },
  );
});
