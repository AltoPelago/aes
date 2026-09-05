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
const portableSpec = readFileSync(new URL('../specifications/aes.events.md', import.meta.url), 'utf8');
const portableContract = portableSpec.match(/^Contract identifier: `([^`]+)`$/mu)?.[1];
const referenceSource = readFileSync(new URL('../src/telex.js', import.meta.url), 'utf8');
const datatypeSource = readFileSync(new URL('../src/datatype.js', import.meta.url), 'utf8');

test('Telex conformance manifest has resolvable suites and unique vector IDs', () => {
  assert.equal(manifest.meta.format, 'telex.aes');
  assert.equal(manifest.meta.format_version, '0');
  assert.equal(manifest.meta.status, 'draft');
  assert.match(manifest.meta.version, /-dev$/u);
  assert.equal(Object.hasOwn(manifest.meta, 'snapshot_id'), false);
  assert.equal(Object.hasOwn(manifest.meta, 'spec_snapshot_id'), false);
  assert.notEqual(portableContract, undefined);
  assert.equal(manifest.meta.event_contract, portableContract);
  assert.ok(Array.isArray(manifest.suites));
  assert.ok(manifest.suites.length > 0);

  for (const suiteRef of manifest.suites) {
    const suite = readJson(new URL(suiteRef.file, manifestUrl));
    assert.equal(suite.id, suiteRef.id);
    assert.equal(suite.meta.event_contract, manifest.meta.event_contract);
    assert.ok(Array.isArray(suite.tests));
    for (const vector of suite.tests) {
      assert.equal(typeof vector.id, 'string');
      assert.equal(seenIds.has(vector.id), false, `duplicate vector id: ${vector.id}`);
      seenIds.add(vector.id);
    }
  }
});

test('conformance specification references resolve to published headings', () => {
  for (const suiteRef of manifest.suites) {
    const suite = readJson(new URL(suiteRef.file, manifestUrl));
    for (const specRef of suite.meta.spec_refs ?? []) {
      const [relativePath, fragment] = specRef.split('#', 2);
      const specification = readFileSync(new URL(`../${relativePath}`, import.meta.url), 'utf8');
      if (fragment !== undefined) {
        const anchors = markdownHeadingAnchors(specification);
        assert.ok(anchors.has(fragment), `missing specification heading: ${specRef}`);
      }
    }
  }
});

test('portable AES vocabulary matches the reference validator', () => {
  const coreBlock = referenceSource.match(/const AES_CORE_FIELDS = \[([\s\S]*?)\];/u)?.[1];
  const kindBlock = referenceSource.match(/const VALUE_KINDS = new Set\(\[([\s\S]*?)\]\);/u)?.[1];
  assert.notEqual(coreBlock, undefined);
  assert.notEqual(kindBlock, undefined);

  const documentedCore = tableFirstColumn(section(portableSpec, '### 3.1 Core fields', '### 3.2 Extensions'));
  const documentedKinds = tableFirstColumn(section(portableSpec, '### 4.2 Value kinds', '### 4.3 References'))
    .filter((value) => value !== 'kind');
  assert.deepEqual(documentedCore, quotedValues(coreBlock));
  assert.deepEqual(new Set(documentedKinds), new Set(quotedValues(kindBlock)));

  const implementedCodes = new Set([...`${referenceSource}\n${datatypeSource}`.matchAll(/['"](AES_[A-Z_]+)['"]/gu)]
    .map((match) => match[1]));
  const documentedCodes = new Set(tableFirstColumn(section(
    portableSpec,
    '### 12.1 Local diagnostics',
    '### 12.2 Source-backed audit diagnostics',
  )));
  assert.deepEqual(documentedCodes, implementedCodes);
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
      projection: parsed.projection,
      projection_explicit: parsed.projectionExplicit,
      canonical: parsed.canonical,
      records: parsed.records,
    }, vector.expected);
  } catch (error) {
    assert.equal(vector.expected.ok, false, `unexpected parse error: ${error.message}`);
    assert.ok(error instanceof TelexSyntaxError);
    assert.deepEqual({ code: error.code, line: error.line ?? null }, vector.expected.error);
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

function section(markdown, start, end) {
  const afterStart = markdown.split(start)[1];
  assert.notEqual(afterStart, undefined, `missing heading: ${start}`);
  const beforeEnd = afterStart.split(end)[0];
  assert.notEqual(beforeEnd, undefined, `missing heading: ${end}`);
  return beforeEnd;
}

function tableFirstColumn(markdown) {
  return [...markdown.matchAll(/^\| `([^`]+)` \|/gmu)]
    .map((match) => match[1]);
}

function quotedValues(source) {
  return [...source.matchAll(/['"]([^'"]+)['"]/gu)]
    .map((match) => match[1]);
}
