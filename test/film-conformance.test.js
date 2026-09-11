import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const manifestUrl = new URL('../conformance/film/v1/film-cts.v1.json', import.meta.url);
const manifest = readJson(manifestUrl);
const specification = readFileSync(new URL('../specifications/film.aes.md', import.meta.url), 'utf8');

test('mutable Film manifest has resolvable suites and unique language-neutral vectors', () => {
  assert.equal(manifest.meta.version, '0.1.0-dev');
  assert.equal(manifest.meta.status, 'draft');
  assert.equal(manifest.meta.lane, 'aes-film');
  assert.equal(manifest.meta.format, 'film.aes');
  assert.equal(manifest.meta.format_version, '1');
  assert.equal(manifest.meta.event_contract, 'aes.events.v1');
  assert.equal(manifest.meta.byte_encoding, 'lowercase-hex');
  assert.equal(Object.hasOwn(manifest.meta, 'snapshot_id'), false);
  assert.equal(Object.hasOwn(manifest.meta, 'spec_snapshot_id'), false);

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
  assert.equal(count, 68);
});

test('mutable Film vectors reference existing specification headings', () => {
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

test('Film specification example is fixed by the mutable scalar vector', () => {
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
