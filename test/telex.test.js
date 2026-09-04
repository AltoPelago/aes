import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  DEFAULT_TELEX_PROFILE,
  RAW_TELEX_PROFILE,
  TelexSyntaxError,
  checkPrefixCompleteness,
  checkTelexCompleteness,
  canonicalizeTelex,
  encodeTelex,
  parseTelex,
} from '../src/telex.js';

test('round-trips records and canonicalizes core field order', () => {
  const records = [{
    value: 'hello\nworld\\again=still-value',
    path: '$.message',
    span: '0:19',
    kind: 'string',
  }];

  const encoded = encodeTelex(records);
  assert.equal(encoded, [
    'telex.aes=0',
    '',
    'path=$.message',
    'kind=string',
    'value=hello\\nworld\\\\again=still-value',
    'span=0:19',
    '',
  ].join('\n'));
  assert.deepEqual(parseTelex(encoded), {
    version: '0',
    profile: DEFAULT_TELEX_PROFILE,
    profileExplicit: false,
    records: [{
      path: '$.message',
      kind: 'string',
      value: 'hello\nworld\\again=still-value',
      span: '0:19',
    }],
    canonical: true,
  });
});

test('preserves Unicode and escapes control scalars', () => {
  const encoded = encodeTelex([{ path: '$.波', kind: 'string', value: '🌊\u0001' }]);
  assert.match(encoded, /value=🌊\\u\{1\}/u);
  assert.equal(parseTelex(encoded).records[0].value, '🌊\u0001');
});

test('keeps clone and pointer references distinct through kind', () => {
  const records = [
    { path: '$.copy', kind: 'clone-reference', value: '$.source' },
    { path: '$.alias', kind: 'pointer-reference', value: '$.source' },
  ];
  assert.deepEqual(parseTelex(encodeTelex(records)).records, records);
});

test('keeps node containers, heads, and content as separate flat events', () => {
  const records = [
    { path: '$.a', kind: 'node', datatype: 'node', identity: 'A' },
    { path: '$.a.@.x', kind: 'number', value: '1' },
    { path: '$.a[0]', kind: 'node-head', datatype: 'node<string>', identity: 'T', value: 'tag' },
    { path: '$.a[0].@.x', kind: 'number', value: '2' },
    { path: '$.a[0][0]', kind: 'string', value: 'hello' },
  ];
  assert.deepEqual(parseTelex(encodeTelex(records)).records, records);
});

test('preserves supplied event order instead of sorting by path', () => {
  const records = [
    { path: '$.z', kind: 'number', value: '1' },
    { path: '$.a', kind: 'number', value: '2' },
    { path: '$.z.@.x', kind: 'string', value: 'third' },
  ];
  const decoded = parseTelex(encodeTelex(records)).records;
  assert.deepEqual(decoded.map(({ path }) => path), ['$.z', '$.a', '$.z.@.x']);
});

test('reports missing structural prefixes without requiring parent-first order', () => {
  const complete = [
    { path: '$.a[0][0]', kind: 'string', value: 'child' },
    { path: '$.a[0]', kind: 'node-head', value: 'tag' },
    { path: '$.a', kind: 'node' },
    { path: '$.a[0].@.["x.y"]', kind: 'number', value: '1' },
  ];
  assert.deepEqual(checkPrefixCompleteness(complete), { complete: true, missing: [] });

  const raw = encodeTelex([{ path: '$.a.b.c', kind: 'number', value: '1' }]);
  assert.deepEqual(checkTelexCompleteness(raw), {
    complete: false,
    missing: [
      { path: '$.a', requiredBy: '$.a.b.c' },
      { path: '$.a.b', requiredBy: '$.a.b.c' },
    ],
  });
});

test('treats an attribute selector and its key as one structural path step', () => {
  const records = [
    { path: '$.a', kind: 'number', value: '0' },
    { path: '$.a.@.meta', kind: 'object' },
    { path: '$.a.@.meta.deep', kind: 'number', value: '1' },
  ];
  assert.equal(checkPrefixCompleteness(records).complete, true);
  assert.deepEqual(checkPrefixCompleteness(records.slice(1)), {
    complete: false,
    missing: [{ path: '$.a', requiredBy: '$.a.@.meta' }],
  });
});

test('preserves unknown extension fields without interpreting them', () => {
  const input = [
    'telex.aes=0',
    '',
    'path=$.a',
    'kind=number',
    'value=1',
    'x.aesdb.revision=42',
    '',
  ].join('\n');
  const parsed = parseTelex(input);
  assert.equal(parsed.records[0]['x.aesdb.revision'], '42');
  assert.equal(encodeTelex(parsed.records), input);
});

test('defaults to the complete Telex profile when no profile is declared', () => {
  const parsed = parseTelex(encodeTelex([{ path: '$.a', kind: 'object' }]));
  assert.equal(parsed.profile, DEFAULT_TELEX_PROFILE);
  assert.equal(parsed.profileExplicit, false);
});

test('round-trips an explicit raw profile without treating it as an event', () => {
  const records = [{ path: '$.a.b', kind: 'number', value: '1' }];
  const encoded = encodeTelex(records, { profile: RAW_TELEX_PROFILE });
  assert.equal(encoded, [
    'telex.aes=0',
    'profile=aes.raw.v0',
    '',
    'path=$.a.b',
    'kind=number',
    'value=1',
    '',
  ].join('\n'));
  assert.deepEqual(parseTelex(encoded), {
    version: '0',
    profile: RAW_TELEX_PROFILE,
    profileExplicit: true,
    records,
    canonical: true,
  });
  assert.equal(canonicalizeTelex(encoded), encoded);
});

test('accepts unknown non-empty profiles at the syntax layer', () => {
  const input = 'telex.aes=0\nprofile=x.example.future.v1\n';
  assert.deepEqual(parseTelex(input), {
    version: '0',
    profile: 'x.example.future.v1',
    profileExplicit: true,
    records: [],
    canonical: true,
  });
});

test('rejects an empty profile declaration', () => {
  assert.throws(
    () => parseTelex('telex.aes=0\nprofile=\n'),
    (error) => error instanceof TelexSyntaxError && /must not be empty/u.test(error.message),
  );
  assert.throws(
    () => encodeTelex([], { profile: '' }),
    /must be a non-empty string/u,
  );
});

test('accepts tolerant syntax and exposes that it is non-canonical', () => {
  const input = 'telex.aes=0\r\n\r\nkind=string\r\npath=$.x\r\nvalue=\\u{000041}\r\n';
  const parsed = parseTelex(input);
  assert.equal(parsed.records[0].value, 'A');
  assert.equal(parsed.canonical, false);
  assert.equal(canonicalizeTelex(input), 'telex.aes=0\n\npath=$.x\nkind=string\nvalue=A\n');
});

test('rejects duplicate fields', () => {
  assert.throws(
    () => parseTelex('telex.aes=0\n\npath=$.x\npath=$.y\n'),
    (error) => error instanceof TelexSyntaxError && /Duplicate field/u.test(error.message),
  );
});

test('rejects unknown escapes and bare carriage returns', () => {
  assert.throws(
    () => parseTelex('telex.aes=0\n\npath=$.assign\\qment\n'),
    (error) => error instanceof TelexSyntaxError && /Unknown escape/u.test(error.message),
  );
  assert.throws(
    () => parseTelex('telex.aes=0\r\npath=$.x\r'),
    (error) => error instanceof TelexSyntaxError && /Bare carriage returns/u.test(error.message),
  );
});

test('supports an empty stream', () => {
  assert.equal(encodeTelex([]), 'telex.aes=0\n');
  assert.deepEqual(parseTelex('telex.aes=0\n'), {
    version: '0',
    profile: DEFAULT_TELEX_PROFILE,
    profileExplicit: false,
    records: [],
    canonical: true,
  });
  assert.equal(parseTelex('telex.aes=0\n\n').canonical, false);
});

test('keeps the repository example canonical', () => {
  const source = readFileSync(new URL('../examples/customer.telex.aes', import.meta.url), 'utf8');
  assert.equal(parseTelex(source).canonical, true);
  assert.equal(canonicalizeTelex(source), source);
});

test('rejects surrogate code units', () => {
  assert.throws(
    () => encodeTelex([{ path: '$.x', value: '\uD800' }]),
    /Unicode scalar values/u,
  );
  assert.throws(
    () => parseTelex('telex.aes=0\n\npath=$.x\nvalue=\uD800\n'),
    /surrogate/u,
  );
});
