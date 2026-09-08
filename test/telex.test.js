import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  AEON_DOCUMENT_PROJECTION,
  COMPLETE_AES_PROFILE,
  DEFAULT_TELEX_LIMITS,
  PARTIAL_AES_PROFILE,
  TelexSyntaxError,
  checkPrefixCompleteness,
  checkTelexCompleteness,
  canonicalizeTelex,
  encodeTelex,
  parseTelex,
  validateTelex,
  validateTelexRecords,
} from '../src/telex.js';
import { formatDatatypeDescriptor, parseDatatypeDescriptor } from '../src/datatype.js';

test('publishes the AltoPelago Telex limit defaults', () => {
  assert.deepEqual(DEFAULT_TELEX_LIMITS, {
    maxInputBytes: 67_108_864,
    maxLineBytes: 1_048_576,
    maxFieldsPerEvent: 64,
    maxEvents: 100_000,
    maxDecodedPayloadBytes: 33_554_432,
    maxPathDepth: 1_024,
    maxPathCharacters: 8_192,
    maxAttributeDepth: 1,
    maxValueNestingDepth: 256,
    maxStringCodepoints: 1_048_576,
    maxKeySegmentCodepoints: 1_024,
    maxListItems: 65_536,
    maxTupleItems: 65_536,
    maxGenericDepth: 1,
    maxGenericArguments: 32,
    maxClarifierValues: 1,
    maxDatatypeComponents: 64,
  });
  assert.throws(
    () => parseTelex('telex.aes=0\n', { limits: { maxInputBytes: 11 } }),
    (error) => error instanceof TelexSyntaxError
      && error.code === 'TELEX_LIMIT_EXCEEDED'
      && error.counter === 'max_input_bytes'
      && error.observed === 12
      && error.limit === 11,
  );
});

test('enforces shared structural limits on direct portable records', () => {
  const cases = [
    {
      counter: 'max_attribute_depth',
      records: [{ path: '$.a.@.x.@.y', kind: 'NumberLiteral', value: '1' }],
      options: { profile: 'aes.partial.v0', maxAttributeDepth: 1 },
    },
    {
      counter: 'max_value_nesting_depth',
      records: [
        { path: '$.a', kind: 'ObjectNode' },
        { path: '$.a.b', kind: 'ObjectNode' },
      ],
      options: { maxValueNestingDepth: 1 },
    },
    {
      counter: 'max_string_codepoints',
      records: [{ path: '$.a', kind: 'StringLiteral', value: '😀x' }],
      options: { maxStringCodepoints: 1 },
    },
    {
      counter: 'max_key_segment_codepoints',
      records: [{ path: '$.["😀x"]', kind: 'NumberLiteral', value: '1' }],
      options: { maxKeySegmentCodepoints: 1 },
    },
    {
      counter: 'max_list_items',
      records: [
        { path: '$.a', kind: 'ListNode' },
        { path: '$.a[0]', kind: 'NumberLiteral', value: '1' },
        { path: '$.a[1]', kind: 'NumberLiteral', value: '2' },
      ],
      options: { maxListItems: 1 },
    },
    {
      counter: 'max_tuple_items',
      records: [
        { path: '$.a', kind: 'TupleLiteral' },
        { path: '$.a[0]', kind: 'NumberLiteral', value: '1' },
        { path: '$.a[1]', kind: 'NumberLiteral', value: '2' },
      ],
      options: { maxTupleItems: 1 },
    },
  ];
  for (const item of cases) {
    const result = validateTelexRecords(item.records, item.options);
    assert.equal(result.valid, false, item.counter);
    assert.ok(result.diagnostics.some((diagnostic) => diagnostic.counter === item.counter), item.counter);
    assert.throws(
      () => encodeTelex(item.records, item.options),
      (error) => error?.code === 'TELEX_LIMIT_EXCEEDED' && error.counter === item.counter,
      item.counter,
    );
  }
});

test('round-trips records and canonicalizes core field order', () => {
  const records = [{
    value: 'hello\nworld\\again=still-value',
    path: '$.message',
    span: '0:19',
    origin: 'sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
    kind: 'StringLiteral',
  }];

  const encoded = encodeTelex(records);
  assert.equal(encoded, [
    'telex.aes=0',
    '',
    'path=$.message',
    'kind=StringLiteral',
    'value=hello\\nworld\\\\again=still-value',
    'origin=sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
    'span=0:19',
    '',
  ].join('\n'));
  assert.deepEqual(parseTelex(encoded), {
    version: '0',
    profile: COMPLETE_AES_PROFILE,
    profileExplicit: false,
    projection: null,
    projectionExplicit: false,
    records: [{
      path: '$.message',
      kind: 'StringLiteral',
      value: 'hello\nworld\\again=still-value',
      origin: 'sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
      span: '0:19',
    }],
    canonical: true,
  });
});

test('preserves Unicode and escapes control scalars', () => {
  const encoded = encodeTelex([{ path: '$.波', kind: 'StringLiteral', value: '🌊\u0001' }]);
  assert.match(encoded, /value=🌊\\u\{1\}/u);
  assert.equal(parseTelex(encoded).records[0].value, '🌊\u0001');
});

test('keeps clone and pointer references distinct through kind', () => {
  const records = [
    { path: '$.copy', kind: 'CloneReference', value: '$.source' },
    { path: '$.alias', kind: 'PointerReference', value: '$.source' },
  ];
  assert.deepEqual(parseTelex(encodeTelex(records)).records, records);
});

test('keeps node containers, heads, and content as separate flat events', () => {
  const records = [
    { path: '$.a', kind: 'NodeLiteral', datatype: 'node', generics: [], clarifiers: [], identity: 'A' },
    { path: '$.a.@.x', kind: 'NumberLiteral', value: '1' },
    {
      path: '$.a[0]',
      kind: 'NodeHead',
      datatype: 'node',
      generics: [{ datatype: 'string', generics: [], clarifiers: [] }],
      clarifiers: [],
      identity: 'T',
      value: 'tag',
    },
    { path: '$.a[0].@.x', kind: 'NumberLiteral', value: '2' },
    { path: '$.a[0][0]', kind: 'StringLiteral', value: 'hello' },
  ];
  assert.deepEqual(parseTelex(encodeTelex(records)).records, records);
});

test('expands and recombines datatype metadata at the Telex boundary', () => {
  const records = [
    {
      path: '$.csv',
      kind: 'SeparatorLiteral',
      datatype: 'csv',
      generics: [],
      clarifiers: [{ kind: 'StringLiteral', value: '.' }],
      value: 'one.two.three',
    },
    {
      path: '$.nested',
      kind: 'ListNode',
      datatype: 'list',
      generics: [{
        datatype: 'tuple',
        generics: [
          { datatype: 'int', generics: [], clarifiers: [] },
          { kind: 'NumberLiteral', value: '9007199254740993' },
        ],
        clarifiers: [{ kind: 'NumberLiteral', value: '16' }],
      }],
      clarifiers: [],
    },
  ];
  const encoded = encodeTelex(records);
  assert.match(encoded, /datatype=csv\["\."\]/u);
  assert.match(encoded, /datatype=list<tuple<int, 9007199254740993>\[16\]>/u);
  assert.deepEqual(parseTelex(encoded).records, records);
});

test('datatype descriptor helpers preserve ordered duplicate clarifiers', () => {
  const limits = { maxClarifierValues: 3 };
  const descriptor = parseDatatypeDescriptor('grid<tuple<int, 2>>["x", "x", 16]', limits);
  assert.deepEqual(descriptor.clarifiers, [
    { kind: 'StringLiteral', value: 'x' },
    { kind: 'StringLiteral', value: 'x' },
    { kind: 'NumberLiteral', value: '16' },
  ]);
  assert.equal(formatDatatypeDescriptor(descriptor, limits), 'grid<tuple<int, 2>>["x", "x", 16]');
});

test('bounds recursive datatype decoding', () => {
  assert.throws(
    () => parseDatatypeDescriptor('list<list<int>>', { maxDepth: 0 }),
    /max_generic_depth .* exceeds configured limit/u,
  );
  assert.throws(
    () => parseDatatypeDescriptor('list<list<list<int>>>'),
    /max_generic_depth .* exceeds configured limit/u,
  );
  assert.equal(
    formatDatatypeDescriptor(
      parseDatatypeDescriptor('list<list<list<int>>>', { maxDepth: 2 }),
      { maxDepth: 2 },
    ),
    'list<list<list<int>>>',
  );
  assert.throws(
    () => parseDatatypeDescriptor('tuple<int, int>', { maxItems: 2 }),
    /max_datatype_components .* exceeds configured limit/u,
  );
});

test('requires complete logical datatype components', () => {
  const missing = validateTelexRecords([{
    path: '$.value',
    kind: 'NumberLiteral',
    datatype: 'int',
    value: '1',
  }]);
  assert.deepEqual(missing.diagnostics.map(({ code }) => code), ['AES_DATATYPE_COMPONENTS']);
  assert.throws(
    () => encodeTelex([{
      path: '$.value',
      kind: 'NumberLiteral',
      datatype: 'int',
      value: '1',
    }]),
    /requires generics and clarifiers arrays/u,
  );
});

test('rejects malformed logical datatype trees', () => {
  const result = validateTelexRecords([{
    path: '$.value',
    kind: 'NumberLiteral',
    datatype: 'measure',
    generics: [{ kind: 'NumberLiteral', value: 'not-a-number' }],
    clarifiers: [],
    value: '1',
  }]);
  assert.deepEqual(result.diagnostics.map(({ code }) => code), ['AES_INVALID_DATATYPE']);
});

test('applies the default logical datatype depth limit', () => {
  const leaf = { datatype: 'int', generics: [], clarifiers: [] };
  const nested = {
    datatype: 'list',
    generics: [{
      datatype: 'list',
      generics: [{ datatype: 'list', generics: [leaf], clarifiers: [] }],
      clarifiers: [],
    }],
    clarifiers: [],
  };
  const event = {
    path: '$.value',
    kind: 'ListNode',
    ...nested,
  };
  assert.deepEqual(
    validateTelexRecords([event]).diagnostics.map(({ code }) => code),
    ['AES_DATATYPE_DEPTH'],
  );
  assert.doesNotThrow(() => encodeTelex([event], { datatypeLimits: { maxDepth: 3 } }));
});

test('preserves supplied event order instead of sorting by path', () => {
  const records = [
    { path: '$.z', kind: 'NumberLiteral', value: '1' },
    { path: '$.a', kind: 'NumberLiteral', value: '2' },
    { path: '$.z.@.x', kind: 'StringLiteral', value: 'third' },
  ];
  const decoded = parseTelex(encodeTelex(records)).records;
  assert.deepEqual(decoded.map(({ path }) => path), ['$.z', '$.a', '$.z.@.x']);
});

test('reports missing structural prefixes without requiring parent-first order', () => {
  const complete = [
    { path: '$.a[0][0]', kind: 'StringLiteral', value: 'child' },
    { path: '$.a[0]', kind: 'NodeHead', value: 'tag' },
    { path: '$.a', kind: 'NodeLiteral' },
    { path: '$.a[0].@.["x.y"]', kind: 'NumberLiteral', value: '1' },
  ];
  assert.deepEqual(checkPrefixCompleteness(complete), { complete: true, missing: [] });

  const incomplete = encodeTelex([{ path: '$.a.b.c', kind: 'NumberLiteral', value: '1' }]);
  assert.deepEqual(checkTelexCompleteness(incomplete), {
    complete: false,
    missing: [
      { path: '$.a', requiredBy: '$.a.b.c' },
      { path: '$.a.b', requiredBy: '$.a.b.c' },
    ],
  });
});

test('applies caller-selected parser limits during completeness checks', () => {
  const source = 'telex.aes=0\n\npath=$.answer\nkind=NumberLiteral\nvalue=42\n';
  assert.throws(
    () => checkTelexCompleteness(source, { maxEvents: 0 }),
    (error) => error?.code === 'TELEX_LIMIT_EXCEEDED',
  );
});

test('treats an attribute selector and its key as one structural path step', () => {
  const records = [
    { path: '$.a', kind: 'NumberLiteral', value: '0' },
    { path: '$.a.@.meta', kind: 'ObjectNode' },
    { path: '$.a.@.meta.deep', kind: 'NumberLiteral', value: '1' },
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
    'kind=NumberLiteral',
    'value=1',
    'x.aesdb.revision=42',
    '',
  ].join('\n');
  const parsed = parseTelex(input);
  assert.equal(parsed.records[0]['x.aesdb.revision'], '42');
  assert.equal(encodeTelex(parsed.records), input);
});

test('defaults to the complete Telex profile when no profile is declared', () => {
  const parsed = parseTelex(encodeTelex([{ path: '$.a', kind: 'ObjectNode' }]));
  assert.equal(parsed.profile, COMPLETE_AES_PROFILE);
  assert.equal(parsed.profileExplicit, false);
});

test('round-trips an explicit partial profile without treating it as an event', () => {
  const records = [{ path: '$.a.b', kind: 'NumberLiteral', value: '1' }];
  const encoded = encodeTelex(records, { profile: PARTIAL_AES_PROFILE });
  assert.equal(encoded, [
    'telex.aes=0',
    'profile=aes.partial.v0',
    '',
    'path=$.a.b',
    'kind=NumberLiteral',
    'value=1',
    '',
  ].join('\n'));
  assert.deepEqual(parseTelex(encoded), {
    version: '0',
    profile: PARTIAL_AES_PROFILE,
    profileExplicit: true,
    projection: null,
    projectionExplicit: false,
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
    projection: null,
    projectionExplicit: false,
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

test('round-trips an explicit AEON document projection with flat header records', () => {
  const records = [
    { header: '$.["aeon:mode"]', kind: 'StringLiteral', value: 'strict' },
    { path: '$.a', kind: 'NumberLiteral', value: '1' },
  ];
  const encoded = encodeTelex(records, { projection: AEON_DOCUMENT_PROJECTION });
  assert.equal(encoded, [
    'telex.aes=0',
    'projection=aeon.document.v0',
    '',
    'header=$.["aeon:mode"]',
    'kind=StringLiteral',
    'value=strict',
    '',
    'path=$.a',
    'kind=NumberLiteral',
    'value=1',
    '',
  ].join('\n'));
  const parsed = parseTelex(encoded);
  assert.equal(parsed.projection, AEON_DOCUMENT_PROJECTION);
  assert.equal(parsed.projectionExplicit, true);
  assert.deepEqual(parsed.records, records);
  assert.equal(validateTelex(encoded).valid, true);
});

test('keeps nested header completeness separate from body completeness', () => {
  const valid = validateTelexRecords([
    { header: '$.["aeon:conventions"]', kind: 'ListNode' },
    { header: '$.["aeon:conventions"][0]', kind: 'StringLiteral', value: 'aeon.gp.security.v1' },
    { path: '$.a.b', kind: 'NumberLiteral', value: '1' },
  ], { profile: PARTIAL_AES_PROFILE, projection: AEON_DOCUMENT_PROJECTION });
  assert.equal(valid.valid, true);

  const incomplete = validateTelexRecords([
    { header: '$.["aeon:conventions"][0]', kind: 'StringLiteral', value: 'aeon.gp.security.v1' },
  ], { profile: PARTIAL_AES_PROFILE, projection: AEON_DOCUMENT_PROJECTION });
  assert.deepEqual(incomplete.diagnostics.map(({ code }) => code), ['AES_MISSING_PARENT']);
});

test('rejects implicit, ambiguous, and late header records', () => {
  assert.deepEqual(validateTelexRecords([
    { header: '$.["aeon:mode"]', kind: 'StringLiteral', value: 'strict' },
  ]).diagnostics.map(({ code }) => code), ['AES_HEADER_REQUIRES_PROJECTION']);

  const invalid = validateTelexRecords([
    { path: '$.a', kind: 'NumberLiteral', value: '1' },
    { header: '$.["aeon:mode"]', kind: 'StringLiteral', value: 'strict' },
    { path: '$.b', header: '$.["aeon:profile"]', kind: 'StringLiteral', value: 'x' },
  ], { projection: AEON_DOCUMENT_PROJECTION });
  assert.deepEqual(invalid.diagnostics.map(({ code }) => code), [
    'AES_HEADER_ORDER',
    'AES_MULTIPLE_ADDRESSES',
  ]);
});

test('validates a complete flat stream under the default profile', () => {
  const records = [
    { path: '$.a', kind: 'ObjectNode', identity: 'root-a' },
    { path: '$.a.@.source', kind: 'StringLiteral', value: 'fixture' },
    { path: '$.a.items', kind: 'ListNode' },
    { path: '$.a.items[0]', kind: 'NodeLiteral' },
    { path: '$.a.items[0][0]', kind: 'NodeHead', value: 'tag' },
    {
      path: '$.a.items[0][0][0]',
      kind: 'BooleanLiteral',
      value: 'true',
      origin: 'sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
      span: '8:12',
    },
  ];
  assert.deepEqual(validateTelexRecords(records), {
    valid: true,
    profile: COMPLETE_AES_PROFILE,
    diagnostics: [],
  });
});

test('requires ancestry in the default profile but not the partial profile', () => {
  const records = [{ path: '$.a.b', kind: 'NumberLiteral', value: '1' }];
  const complete = validateTelexRecords(records);
  assert.equal(complete.valid, false);
  assert.equal(complete.diagnostics[0].code, 'AES_MISSING_PARENT');

  assert.deepEqual(validateTelexRecords(records, { profile: PARTIAL_AES_PROFILE }), {
    valid: true,
    profile: PARTIAL_AES_PROFILE,
    diagnostics: [],
  });
});

test('does not treat the unrepresented root as an indexed container or attribute owner', () => {
  const result = validateTelexRecords([
    { path: '$[0]', kind: 'NumberLiteral', value: '1' },
    { path: '$.@.meta', kind: 'StringLiteral', value: 'root' },
  ]);
  assert.deepEqual(result.diagnostics.map(({ code }) => code), [
    'AES_MISSING_PARENT',
    'AES_MISSING_PARENT',
  ]);
});

test('allows repeated paths and identities only in the partial profile', () => {
  const records = [
    { path: '$.a', kind: 'NumberLiteral', identity: 'same', value: '1' },
    { path: '$.a', kind: 'NumberLiteral', identity: 'same', value: '2' },
  ];
  const completeCodes = validateTelexRecords(records).diagnostics.map(({ code }) => code);
  assert.deepEqual(completeCodes, ['AES_DUPLICATE_PATH', 'AES_DUPLICATE_IDENTITY']);
  assert.equal(validateTelexRecords(records, { profile: PARTIAL_AES_PROFILE }).valid, true);
});

test('retains event-local validation in the partial profile', () => {
  const result = validateTelexRecords([
    { path: '$.a', kind: 'ObjectNode', value: 'nested' },
    { path: '$.b', kind: 'BooleanLiteral', value: 'True' },
    { path: '$.c', kind: 'future-kind' },
    { path: '$.d', kind: 'StringLiteral', 'x.example.claim': 'yes' },
  ], { profile: PARTIAL_AES_PROFILE });
  assert.equal(result.valid, false);
  assert.deepEqual(result.diagnostics.map(({ code }) => code), [
    'AES_UNEXPECTED_VALUE',
    'AES_INVALID_VALUE',
    'AES_UNKNOWN_KIND',
    'AES_UNKNOWN_FIELD',
    'AES_MISSING_VALUE',
  ]);
});

test('checks parent kinds and node-head placement in the complete profile', () => {
  const result = validateTelexRecords([
    { path: '$.scalar', kind: 'NumberLiteral', value: '1' },
    { path: '$.scalar.child', kind: 'StringLiteral', value: 'no' },
    { path: '$.list', kind: 'ListNode' },
    { path: '$.list[0]', kind: 'NodeHead', value: 'misplaced' },
    { path: '$.node', kind: 'NodeLiteral' },
    { path: '$.node[0]', kind: 'StringLiteral', value: 'not-a-head' },
  ]);
  assert.deepEqual(result.diagnostics.map(({ code }) => code), [
    'AES_INCOMPATIBLE_PARENT',
    'AES_INVALID_NODE_HEAD',
    'AES_INCOMPATIBLE_PARENT',
  ]);
});

test('checks locally specified payload and span grammars', () => {
  const result = validateTelexRecords([
    { path: '$.hex', kind: 'HexLiteral', value: 'CAFE' },
    { path: '$.ref', kind: 'CloneReference', value: 'relative.path' },
    {
      path: '$.span',
      kind: 'NaNLiteral',
      value: 'nan',
      origin: 'sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
      span: '4:2',
    },
    { path: '$.empty', kind: 'NumberLiteral' },
  ], { profile: PARTIAL_AES_PROFILE });
  assert.deepEqual(result.diagnostics.map(({ code }) => code), [
    'AES_INVALID_VALUE',
    'AES_INVALID_REFERENCE',
    'AES_INVALID_VALUE',
    'AES_INVALID_SPAN',
    'AES_MISSING_VALUE',
  ]);
});

test('allows only explicitly registered extension fields', () => {
  const records = [{
    path: '$.a',
    kind: 'NumberLiteral',
    value: '1',
    'x.example.claim': 'yes',
  }];
  assert.equal(validateTelexRecords(records, { profile: PARTIAL_AES_PROFILE }).valid, false);
  assert.equal(validateTelexRecords(records, {
    profile: PARTIAL_AES_PROFILE,
    registeredFields: ['x.example.claim'],
  }).valid, true);
});

test('validates the profile selected by a Telex stream', () => {
  const partial = encodeTelex(
    [{ path: '$.a.b', kind: 'NumberLiteral', value: '1' }],
    { profile: PARTIAL_AES_PROFILE },
  );
  assert.equal(validateTelex(partial).valid, true);
  assert.equal(validateTelex(encodeTelex([{ path: '$.a.b', kind: 'NumberLiteral', value: '1' }])).valid, false);
});

test('accepts tolerant syntax and exposes that it is non-canonical', () => {
  const input = 'telex.aes=0\r\n\r\nkind=StringLiteral\r\npath=$.x\r\nvalue=\\u{000041}\r\n';
  const parsed = parseTelex(input);
  assert.equal(parsed.records[0].value, 'A');
  assert.equal(parsed.canonical, false);
  assert.equal(canonicalizeTelex(input), 'telex.aes=0\n\npath=$.x\nkind=StringLiteral\nvalue=A\n');
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
    profile: COMPLETE_AES_PROFILE,
    profileExplicit: false,
    projection: null,
    projectionExplicit: false,
    records: [],
    canonical: true,
  });
  assert.equal(parseTelex('telex.aes=0\n\n').canonical, false);
});

test('keeps the repository example canonical', () => {
  const source = readFileSync(new URL('../examples/customer.telex.aes', import.meta.url), 'utf8');
  assert.equal(parseTelex(source).canonical, true);
  assert.equal(canonicalizeTelex(source), source);
  assert.equal(validateTelex(source).valid, true);
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
