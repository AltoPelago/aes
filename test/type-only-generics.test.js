import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { assertDatatypeDescriptor, formatDatatypeDescriptor, parseDatatypeDescriptor } from '../src/datatype.js';
import { encodeTelex, parseTelex, validateTelexRecords } from '../src/telex.js';
import { decodeFilm, decodeFilmSyntax, FilmDecodeError, IncrementalFilmDecoder } from '../src/film.js';

const type = (datatype, generics = [], clarifiers = []) => ({ datatype, generics, clarifiers });

test('literal generic arguments fail descriptor, logical record, and Telex writer boundaries', () => {
  for (const literal of [
    { kind: 'NumberLiteral', value: '2' },
    { kind: 'NumberLiteral', value: '9007199254740993' },
    { kind: 'StringLiteral', value: 'int' },
  ]) {
    for (const generics of [[literal], [type('nested', [literal])]]) {
      const descriptor = type('custom', generics);
      assert.throws(() => assertDatatypeDescriptor(descriptor), /generic argument must be a datatype descriptor/u);
      assert.throws(() => formatDatatypeDescriptor(descriptor), /generic argument must be a datatype descriptor/u);
      const record = { path: '$.a', kind: 'NumberLiteral', ...descriptor, value: '0' };
      assert.equal(validateTelexRecords([record]).valid, false);
      assert.throws(() => encodeTelex([record]));
    }
  }
  for (const value of ['2', '1.0', '-2', '1e3', '"int"', 'nested<2>']) {
    assert.throws(() => parseDatatypeDescriptor(`custom<${value}>`));
    assert.throws(() => parseTelex(`telex.aes=1\n\npath=$.a\nkind=NumberLiteral\ndatatype=custom<${value}>\nvalue=0\n`));
  }
});

test('exact numeric clarifiers remain valid on recursive type arguments', () => {
  const surface = 'custom<nested[9007199254740993]>[2.0000000000000001]';
  const descriptor = parseDatatypeDescriptor(surface);
  assert.equal(formatDatatypeDescriptor(descriptor), surface);
  const record = { path: '$.a', kind: 'NumberLiteral', ...descriptor, value: '0' };
  assert.deepEqual(parseTelex(encodeTelex([record])).records, [record]);
});

const suite = JSON.parse(readFileSync(new URL('../conformance/film/v1/suites/05-next-type-only-generics.json', import.meta.url), 'utf8'));
for (const fixture of suite.tests.filter(f => f.operation === 'decode'
  && f.expected.error?.diagnostic_codes?.includes('AES_INVALID_DATATYPE'))) {
  test(`provisional Film literals never become accepted across any split: ${fixture.id}`, () => {
    const bytes = Uint8Array.from(Buffer.from(fixture.input.film_hex, 'hex'));
    const options = { limits: {
      maxGenericArguments: fixture.input.limits?.max_generic_arguments ?? 32,
      maxClarifierValues: fixture.input.limits?.max_clarifier_values ?? 1,
    } };
    // Physical framing remains readable, but is not logical AES acceptance.
    assert.ok(decodeFilmSyntax(bytes, options).records.length);
    const rejected = error => error instanceof FilmDecodeError
      && error.code === 'FILM_AES_INVALID'
      && error.diagnostics.some(diagnostic => diagnostic.code === 'AES_INVALID_DATATYPE');
    assert.throws(() => decodeFilm(bytes, options), rejected);
    for (let split = 0; split <= bytes.length; split += 1) {
      const decoder = new IncrementalFilmDecoder(options);
      assert.notEqual(decoder.push(bytes.subarray(0, split)).status, 'complete');
      assert.throws(() => decoder.push(bytes.subarray(split), { final: true }), rejected);
    }
  });
}
