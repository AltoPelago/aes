import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';
import { pathToFileURL } from 'node:url';

import { validateTelexRecords } from '../src/telex.js';

const manifestUrl = process.env.AES_EVENTS_CTS_MANIFEST === undefined
  ? new URL('../../../aeonite-org/aeonite-cts/cts/aes/v0/aes-events-cts.v0.snapshot-0.1.json', import.meta.url)
  : pathToFileURL(resolve(process.env.AES_EVENTS_CTS_MANIFEST));
const manifest = readJson(manifestUrl);

test('published portable AES event manifest is immutable and internally consistent', () => {
  assert.equal(manifest.meta.status, 'released');
  assert.equal(manifest.meta.lane, 'aes-events');
  assert.equal(manifest.meta.event_contract, 'aes.events.v0');
  assert.match(manifest.meta.snapshot_id, /^aes-events-cts-v0-snapshot-\d+\.\d+$/u);
  assert.match(manifest.meta.spec_snapshot_id, /^aes-events-specs-v0-snapshot-\d+\.\d+$/u);
  assert.ok(Array.isArray(manifest.suites));
});

let vectorCount = 0;
const seenIds = new Set();
for (const suiteRef of manifest.suites) {
  const suite = readJson(new URL(suiteRef.file, manifestUrl));
  assert.equal(suite.id, suiteRef.id);
  assert.equal(suite.meta.event_contract, manifest.meta.event_contract);
  for (const vector of suite.tests) {
    vectorCount += 1;
    assert.equal(seenIds.has(vector.id), false, `duplicate vector id: ${vector.id}`);
    seenIds.add(vector.id);
    test(vector.id, () => runVector(vector));
  }
}

test('published portable AES event snapshot contains 38 vectors', () => {
  assert.equal(vectorCount, 38);
});

function runVector(vector) {
  assert.equal(vector.operation, 'validate');
  const result = validateTelexRecords(vector.input.records, {
    ...(vector.input.profile === undefined ? {} : { profile: vector.input.profile }),
    ...(vector.input.projection === undefined ? {} : { projection: vector.input.projection }),
    registeredFields: vector.input.registered_fields ?? [],
  });
  assert.deepEqual({
    valid: result.valid,
    profile: result.profile,
    diagnostic_codes: result.diagnostics.map(({ code }) => code).sort(),
  }, {
    ...vector.expected,
    diagnostic_codes: [...vector.expected.diagnostic_codes].sort(),
  });
}

function readJson(url) {
  return JSON.parse(readFileSync(url, 'utf8'));
}
