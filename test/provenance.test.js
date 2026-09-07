import assert from 'node:assert/strict';
import test from 'node:test';

import { Buffer } from 'node:buffer';

import { auditAesSourceProvenance } from '../src/provenance.js';

const source = Buffer.from('a = "é"\r\n');
const origin = 'sha256:d7f871f7c49226de258ccf11674f66e7395ab65f2e006e42851cd232b03c28e2';

test('audits exact source bytes and non-ASCII UTF-8 span boundaries', () => {
  const result = auditAesSourceProvenance([
    { path: '$.a', kind: 'StringLiteral', value: 'é', origin, span: '4:8' },
  ], new Map([[origin, source]]), { requireAllRecords: true, requireAvailable: true });
  assert.deepEqual(result, {
    valid: true,
    complete: true,
    verifiedOrigins: [origin],
    missingOrigins: [],
    diagnostics: [],
  });
});

test('distinguishes unavailable artifacts from invalid evidence', () => {
  const optional = auditAesSourceProvenance([
    { path: '$.a', kind: 'StringLiteral', value: 'é', origin },
  ], new Map());
  assert.equal(optional.valid, true);
  assert.equal(optional.complete, false);
  assert.deepEqual(optional.missingOrigins, [origin]);

  const required = auditAesSourceProvenance([
    { path: '$.a', kind: 'StringLiteral', value: 'é', origin },
  ], new Map(), { requireAvailable: true });
  assert.equal(required.valid, false);
  assert.equal(required.diagnostics[0]?.code, 'AES_SOURCE_REQUIRED');
});

test('rejects digest mismatch, split scalars, invalid UTF-8, and implicit text encoding', () => {
  const cases = [
    [new Map([[origin, Buffer.from('different')]]), { origin }, 'AES_ORIGIN_MISMATCH'],
    [new Map([[origin, source]]), { origin, span: '6:8' }, 'AES_INVALID_SPAN'],
    [new Map([['sha256:a8100ae6aa1940d0b663bb31cd466142ebbdbd5187131b92d93818987832eb89', Buffer.from([0xff])]]), { origin: 'sha256:a8100ae6aa1940d0b663bb31cd466142ebbdbd5187131b92d93818987832eb89' }, 'AES_SOURCE_INVALID_UTF8'],
    [new Map([[origin, 'a = "é"\r\n']]), { origin }, 'AES_SOURCE_ARTIFACT_INVALID'],
  ];
  for (const [artifacts, provenance, code] of cases) {
    const result = auditAesSourceProvenance([
      { path: '$.a', kind: 'StringLiteral', value: 'é', ...provenance },
    ], artifacts, { requireAvailable: true });
    assert.equal(result.valid, false);
    assert.equal(result.diagnostics[0]?.code, code);
  }
});

test('requires origin on every record only for all-record source-backed claims', () => {
  const records = [{ path: '$.a', kind: 'StringLiteral', value: 'x' }];
  assert.equal(auditAesSourceProvenance(records, new Map()).valid, true);
  const result = auditAesSourceProvenance(records, new Map(), { requireAllRecords: true });
  assert.equal(result.valid, false);
  assert.equal(result.complete, false);
  assert.equal(result.diagnostics[0]?.code, 'AES_SOURCE_REQUIRED');
});
