import assert from 'node:assert/strict';
import { Buffer } from 'node:buffer';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { auditAesSourceProvenance } from '../src/provenance.js';

const manifestUrl = new URL('../conformance/provenance/v0/aes-provenance-cts.v0.json', import.meta.url);
const manifest = readJson(manifestUrl);

test('AES provenance candidate manifest is internally consistent', () => {
  assert.equal(manifest.meta.status, 'draft');
  assert.equal(manifest.meta.lane, 'aes-provenance');
  assert.equal(manifest.meta.event_contract, 'aes.events.v0');
  assert.equal(manifest.meta.preparation_contract, 'aes.preparation.source-backed.v0');
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
    test(vector.id, () => {
      const artifacts = new Map(Object.entries(vector.input.artifacts)
        .map(([origin, base64]) => [origin, Buffer.from(base64, 'base64')]));
      const result = auditAesSourceProvenance(vector.input.records, artifacts, {
        requireAllRecords: vector.input.require_all_records === true,
        requireAvailable: vector.input.require_available === true,
      });
      assert.equal(result.valid, vector.expected.valid);
      assert.equal(result.complete, vector.expected.complete);
      assert.deepEqual(result.verifiedOrigins, vector.expected.verified_origins);
      assert.deepEqual(result.diagnostics.map(({ code }) => code), vector.expected.diagnostic_codes);
    });
  }
}

test('AES provenance candidate contains 11 vectors', () => {
  assert.equal(vectorCount, 11);
});

function readJson(url) {
  return JSON.parse(readFileSync(url, 'utf8'));
}
