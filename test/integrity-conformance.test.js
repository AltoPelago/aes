import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  computeAesIntegrityDigest,
  encodeAesSignatureInput,
} from '../src/integrity.js';

const manifestUrl = new URL(
  '../conformance/integrity/v1/aes-integrity-cts.v1.json',
  import.meta.url,
);
const manifest = readJson(manifestUrl);

test('AES integrity candidate manifest is internally consistent', () => {
  assert.equal(manifest.meta.status, 'draft');
  assert.equal(manifest.meta.lane, 'aes-integrity');
  assert.equal(manifest.meta.integrity_contract, 'aes.integrity.v1');
  assert.equal(manifest.meta.event_contract, 'aes.events.v1');
});

let vectorCount = 0;
const seen = new Set();
for (const suiteRef of manifest.suites) {
  const suite = readJson(new URL(suiteRef.file, manifestUrl));
  assert.equal(suite.id, suiteRef.id);
  for (const vector of suite.tests) {
    vectorCount += 1;
    assert.equal(seen.has(vector.id), false, `duplicate vector id: ${vector.id}`);
    seen.add(vector.id);
    test(vector.id, () => runVector(vector));
  }
}

test('AES integrity candidate contains 17 vectors', () => {
  assert.equal(vectorCount, 17);
});

function runVector(vector) {
  if (vector.operation === 'signature-input') {
    const result = encodeAesSignatureInput(vector.input);
    assert.equal(result.bytes.toString('hex'), vector.expected.logical_hex);
    return;
  }
  assert.equal(vector.operation, 'digest');
  const options = {
    ...(vector.input.profile === undefined ? {} : { profile: vector.input.profile }),
    ...(vector.input.projection === undefined ? {} : { projection: vector.input.projection }),
    ordering: vector.input.ordering,
    scope: vector.input.scope,
    provenance: vector.input.provenance,
    registeredFields: vector.input.registered_fields ?? [],
  };
  if (vector.expected.error_code !== undefined) {
    assert.throws(
      () => computeAesIntegrityDigest(vector.input.records, options),
      { code: vector.expected.error_code },
    );
    return;
  }
  const result = computeAesIntegrityDigest(vector.input.records, options);
  assert.equal(result.digest, vector.expected.digest);
  if (vector.expected.logical_hex !== undefined) {
    assert.equal(result.bytes.toString('hex'), vector.expected.logical_hex);
  }
  assert.deepEqual(
    result.input.records.map((record) => record.header ?? record.path),
    vector.expected.record_addresses,
  );
}

function readJson(url) {
  return JSON.parse(readFileSync(url, 'utf8'));
}
