import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';
import { pathToFileURL } from 'node:url';

const manifestUrl = process.env.FILM_CTS_MANIFEST === undefined
  ? new URL('../conformance/film/v1/film-cts.v1.json', import.meta.url)
  : pathToFileURL(resolve(process.env.FILM_CTS_MANIFEST));
const manifest = readJson(manifestUrl);
const specification = readFileSync(new URL('../specifications/film.aes.md', import.meta.url), 'utf8');

test('selected Film manifest has resolvable suites and unique language-neutral vectors', () => {
  if (manifest.meta.status === 'released') {
    assert.equal(manifest.meta.version, '0.1.0');
    assert.equal(manifest.meta.snapshot_id, 'film-cts-v1-snapshot-0.1');
    assert.equal(manifest.meta.spec_snapshot_id, 'film-specs-v1-snapshot-0.1');
  } else {
    assert.equal(manifest.meta.version, '0.1.0-dev');
    assert.equal(manifest.meta.status, 'draft');
    assert.equal(Object.hasOwn(manifest.meta, 'snapshot_id'), false);
    assert.equal(Object.hasOwn(manifest.meta, 'spec_snapshot_id'), false);
  }
  assert.equal(manifest.meta.lane, 'aes-film');
  assert.equal(manifest.meta.format, 'film.aes');
  assert.equal(manifest.meta.format_version, '1');
  assert.equal(manifest.meta.event_contract, 'aes.events.v1');
  assert.equal(manifest.meta.byte_encoding, 'lowercase-hex');
  const seen = new Set();
  let count = 0;
  for (const suiteRef of manifest.suites) {
    const suite = readJson(new URL(suiteRef.file, manifestUrl));
    assert.equal(suite.id, suiteRef.id);
    assert.equal(suite.meta.suite_id, suite.id);
    assert.equal(suite.meta.lane, manifest.meta.lane);
    assert.equal(suite.meta.format_version, manifest.meta.format_version);
    assert.equal(suite.meta.event_contract, manifest.meta.event_contract);
    assert.ok(Array.isArray(suite.tests));
    for (const vector of suite.tests) {
      assert.equal(typeof vector.id, 'string');
      assert.equal(seen.has(vector.id), false, `duplicate vector id: ${vector.id}`);
      seen.add(vector.id);
      assert.ok(['decode', 'encode', 'transcode'].includes(vector.operation), vector.id);
      validateHexFields(vector, vector.id);
      if (vector.operation === 'decode' && vector.expected.ok === true
          && vector.expected.canonical_hex !== undefined) {
        assert.equal(vector.expected.canonical_hex, vector.input.film_hex, vector.id);
      }
      count += 1;
    }
  }
  assert.equal(count, 72);
});

test('selected Film candidate vectors reference existing specification headings', () => {
  if (manifest.meta.status === 'released') return;
  const anchors = markdownHeadingAnchors(specification);
  for (const suiteRef of manifest.suites) {
    const suite = readJson(new URL(suiteRef.file, manifestUrl));
    for (const specRef of suite.meta.spec_refs ?? []) {
      const [relativePath, fragment] = specRef.split('#', 2);
      assert.equal(relativePath, 'specifications/film.aes.md');
      assert.ok(anchors.has(fragment), `missing Film specification heading: ${specRef}`);
    }
  }
});

test('selected Film vectors satisfy the complete v1 protocol shape', () => {
  const syntaxCodes = new Set();
  let aesFailures = 0;
  for (const suiteRef of manifest.suites) {
    const suite = readJson(new URL(suiteRef.file, manifestUrl));
    assert.ok((suite.meta.spec_refs ?? []).length > 0, `${suite.id}: missing spec_refs`);
    for (const vector of suite.tests) {
      assert.ok(vector.description?.length > 0, `${vector.id}: missing description`);
      assert.ok(Array.isArray(vector.tags) && vector.tags.length > 0, `${vector.id}: tags`);
      assert.equal(typeof vector.expected?.ok, 'boolean', `${vector.id}: expected.ok`);
      if (vector.operation === 'decode') {
        assert.equal(typeof vector.input?.film_hex, 'string', `${vector.id}: film_hex`);
        if (vector.expected.ok) {
          assert.ok(
            typeof vector.expected.canonical_hex === 'string'
              || vector.expected.stream !== undefined,
            `${vector.id}: successful decode expectation`,
          );
        } else if (vector.expected.error?.stage === 'aes') {
          assert.ok(vector.expected.error.diagnostic_codes?.length > 0, vector.id);
          aesFailures += 1;
        } else {
          assert.equal(typeof vector.expected.error?.code, 'string', `${vector.id}: error code`);
          syntaxCodes.add(vector.expected.error.code);
        }
      } else if (vector.operation === 'encode') {
        assert.notEqual(vector.input?.stream, undefined, `${vector.id}: stream`);
        assert.equal(
          typeof (vector.expected.ok ? vector.expected.film_hex : vector.expected.error?.code),
          'string',
          `${vector.id}: encode expectation`,
        );
      } else {
        assert.equal(vector.operation, 'transcode');
        assert.equal(typeof vector.input?.telex, 'string', `${vector.id}: telex input`);
        assert.equal(vector.expected.ok, true, `${vector.id}: transcode must succeed`);
        assert.equal(typeof vector.expected.film_hex, 'string', `${vector.id}: film output`);
        assert.equal(typeof vector.expected.telex, 'string', `${vector.id}: telex output`);
      }
    }
  }
  assert.deepEqual(syntaxCodes, new Set([
    'FILM_INTEGER_OVERFLOW',
    'FILM_INVALID_CONTEXT',
    'FILM_INVALID_DATATYPE',
    'FILM_INVALID_EXTENSION',
    'FILM_INVALID_KIND',
    'FILM_INVALID_PREAMBLE',
    'FILM_INVALID_RECORD',
    'FILM_INVALID_UTF8',
    'FILM_LIMIT_EXCEEDED',
    'FILM_NONCANONICAL',
    'FILM_TRUNCATED',
  ]));
  assert.ok(aesFailures > 0);
});

test('Film specification example is fixed by the selected scalar vector', () => {
  const suite = readJson(new URL('suites/01-framing-and-canonicalization.json', manifestUrl));
  const vector = suite.tests.find(({ id }) => id === 'film-framing-005-canonical-scalar');
  assert.notEqual(vector, undefined);
  assert.match(specification, /4F 5F 5F FF 01 00[\s\S]*?12[\s\S]*?00 01/u);
  assert.equal(
    vector.input.film_hex,
    '4f5f5fff010012000109242e6d6573736167650568656c6c6f',
  );
});

function validateHexFields(value, id) {
  if (Array.isArray(value)) {
    for (const item of value) validateHexFields(item, id);
    return;
  }
  if (value === null || typeof value !== 'object') return;
  for (const [name, item] of Object.entries(value)) {
    if (name.endsWith('_hex')) {
      assert.equal(typeof item, 'string', `${id}: ${name}`);
      assert.match(item, /^(?:[0-9a-f]{2})*$/u, `${id}: ${name}`);
    } else {
      validateHexFields(item, id);
    }
  }
}

function readJson(url) {
  return JSON.parse(readFileSync(url, 'utf8'));
}

function markdownHeadingAnchors(markdown) {
  return new Set(markdown
    .split('\n')
    .filter((line) => /^#{1,6}\s+/u.test(line))
    .map((line) => line
      .replace(/^#{1,6}\s+/u, '')
      .replace(/`/gu, '')
      .toLowerCase()
      .replace(/[^\p{L}\p{N}\s-]/gu, '')
      .trim()
      .replace(/\s+/gu, '-')));
}
