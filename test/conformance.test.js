import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  TelexSyntaxError,
  canonicalizeTelex,
  parseTelex,
  validateTelex,
} from '../src/telex.js';

const manifestUrl = new URL('../conformance/telex/v0/telex-cts.v0.json', import.meta.url);
const manifest = readJson(manifestUrl);
const seenIds = new Set();

test('Telex conformance manifest has resolvable suites and unique vector IDs', () => {
  assert.equal(manifest.meta.format, 'telex.aes');
  assert.equal(manifest.meta.format_version, '0');
  assert.ok(Array.isArray(manifest.suites));
  assert.ok(manifest.suites.length > 0);

  for (const suiteRef of manifest.suites) {
    const suite = readJson(new URL(suiteRef.file, manifestUrl));
    assert.equal(suite.id, suiteRef.id);
    assert.ok(Array.isArray(suite.tests));
    for (const vector of suite.tests) {
      assert.equal(typeof vector.id, 'string');
      assert.equal(seenIds.has(vector.id), false, `duplicate vector id: ${vector.id}`);
      seenIds.add(vector.id);
    }
  }
});

for (const suiteRef of manifest.suites) {
  const suite = readJson(new URL(suiteRef.file, manifestUrl));
  for (const vector of suite.tests) {
    test(vector.id, () => runVector(vector));
  }
}

function runVector(vector) {
  if (vector.operation === 'parse') {
    return runParseVector(vector);
  }
  if (vector.operation === 'canonicalize') {
    return runCanonicalizeVector(vector);
  }
  if (vector.operation === 'validate') {
    return runValidateVector(vector);
  }
  assert.fail(`unsupported conformance operation: ${vector.operation}`);
}

function runParseVector(vector) {
  try {
    const parsed = parseTelex(vector.input.telex);
    assert.notEqual(vector.expected.ok, false, 'expected syntax failure but parsing succeeded');
    assert.deepEqual({
      ok: true,
      version: parsed.version,
      profile: parsed.profile,
      profile_explicit: parsed.profileExplicit,
      canonical: parsed.canonical,
      records: parsed.records,
    }, vector.expected);
  } catch (error) {
    assert.equal(vector.expected.ok, false, `unexpected parse error: ${error.message}`);
    assert.ok(error instanceof TelexSyntaxError);
    assert.deepEqual({ code: error.code, line: error.line }, vector.expected.error);
  }
}

function runCanonicalizeVector(vector) {
  try {
    const telex = canonicalizeTelex(vector.input.telex);
    assert.notEqual(vector.expected.ok, false, 'expected syntax failure but canonicalization succeeded');
    assert.deepEqual({ ok: true, telex }, vector.expected);
  } catch (error) {
    assert.equal(vector.expected.ok, false, `unexpected canonicalization error: ${error.message}`);
    assert.ok(error instanceof TelexSyntaxError);
    assert.deepEqual({ code: error.code, line: error.line }, vector.expected.error);
  }
}

function runValidateVector(vector) {
  const result = validateTelex(vector.input.telex, {
    registeredFields: vector.input.registered_fields ?? [],
  });
  const actual = {
    valid: result.valid,
    profile: result.profile,
    diagnostic_codes: result.diagnostics.map(({ code }) => code).sort(),
  };
  const expected = {
    ...vector.expected,
    diagnostic_codes: [...vector.expected.diagnostic_codes].sort(),
  };
  assert.deepEqual(actual, expected);
}

function readJson(url) {
  return JSON.parse(readFileSync(url, 'utf8'));
}
