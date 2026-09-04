import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  TelexSyntaxError,
  canonicalizeTelex,
  encodeTelex,
  parseTelex,
} from '../src/telex.js';

test('round-trips records and canonicalizes core field order', () => {
  const records = [{
    value: 'hello\nworld\\again=still-value',
    path: '$.message',
    span: '1:1:0-1:20:19',
    kind: 'string',
  }];

  const encoded = encodeTelex(records);
  assert.equal(encoded, [
    'telex.aes=0',
    '',
    'path=$.message',
    'kind=string',
    'value=hello\\nworld\\\\again=still-value',
    'span=1:1:0-1:20:19',
    '',
  ].join('\n'));
  assert.deepEqual(parseTelex(encoded), {
    version: '0',
    records: [{
      path: '$.message',
      kind: 'string',
      value: 'hello\nworld\\again=still-value',
      span: '1:1:0-1:20:19',
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
