import assert from 'node:assert/strict';
import test from 'node:test';
import { encodeTelex, parseTelex, validateTelexRecords, canonicalizeTelex } from '../src/telex.js';

test('empty member and attribute paths survive portable Telex round trips', () => {
  const records = [
    { path: '$.[""]', kind: 'StringLiteral', value: '' },
    { path: '$.[""].@.[""]', kind: 'StringLiteral', value: '' },
  ];
  const validation = validateTelexRecords(records, { profile: 'aes.complete.v1' });
  assert.equal(validation.valid, true, JSON.stringify(validation.diagnostics));
  const encoded = encodeTelex(records, { profile: 'aes.complete.v1' });
  const parsed = parseTelex(encoded);
  assert.deepEqual(parsed.records, records);
  assert.equal(canonicalizeTelex(encoded), encoded);
  const duplicates = validateTelexRecords([...records, records[0]], { profile: 'aes.complete.v1' });
  assert.equal(duplicates.valid, false);
  assert.ok(duplicates.diagnostics.some(error => error.code === 'AES_DUPLICATE_PATH'));
});
