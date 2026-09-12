import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  FilmDecodeError,
  IncrementalFilmDecoder,
  decodeFilm,
  decodeFilmSyntax,
} from '../src/film.js';

const manifestUrl = new URL('../conformance/film/v1/film-cts.v1.json', import.meta.url);
const LIMIT_NAMES = Object.freeze({
  max_input_bytes: 'maxInputBytes',
  max_record_bytes: 'maxRecordBytes',
  max_field_bytes: 'maxFieldBytes',
  max_buffered_bytes: 'maxBufferedBytes',
  max_events: 'maxEvents',
  max_path_depth: 'maxPathDepth',
  max_path_characters: 'maxPathCharacters',
  max_attribute_depth: 'maxAttributeDepth',
  max_value_nesting_depth: 'maxValueNestingDepth',
  max_string_codepoints: 'maxStringCodepoints',
  max_key_segment_codepoints: 'maxKeySegmentCodepoints',
  max_list_items: 'maxListItems',
  max_tuple_items: 'maxTupleItems',
  max_generic_depth: 'maxGenericDepth',
  max_generic_arguments: 'maxGenericArguments',
  max_clarifier_values: 'maxClarifierValues',
  max_datatype_components: 'maxDatatypeComponents',
});
const fixtures = positiveFilmFixtures();

test('incremental decoding matches one-shot decoding at every two-chunk boundary', () => {
  let divisions = 0;
  for (const fixture of fixtures) {
    const bytes = fromHex(fixture.hex);
    const expected = decodeFilm(bytes, fixture.options);
    for (let split = 0; split <= bytes.byteLength; split += 1) {
      const decoder = new IncrementalFilmDecoder(fixture.options);
      const first = decoder.push(bytes.subarray(0, split));
      assert.notEqual(first.status, 'complete', `${fixture.id} split ${split}`);
      const completed = decoder.push(bytes.subarray(split), { final: true });
      assert.equal(completed.status, 'complete', `${fixture.id} split ${split}`);
      assert.deepEqual(completed.stream, expected, `${fixture.id} split ${split}`);
      divisions += 1;
    }
  }
  assert.ok(divisions > 500);
});

test('incremental decoding accepts every positive fixture one byte at a time', () => {
  for (const fixture of fixtures) {
    const bytes = fromHex(fixture.hex);
    const expected = decodeFilm(bytes, fixture.options);
    const decoder = new IncrementalFilmDecoder(fixture.options);
    let completed;
    for (let index = 0; index < bytes.byteLength; index += 1) {
      completed = decoder.push(bytes.subarray(index, index + 1), {
        final: index + 1 === bytes.byteLength,
      });
    }
    assert.equal(completed.status, 'complete', fixture.id);
    assert.deepEqual(completed.stream, expected, fixture.id);
  }
});

test('reports provisional records but withholds completed-stream acceptance', () => {
  const bytes = fromHex('4f5f5fff01000f00010a6e6f742d612d706174680178');
  const decoder = new IncrementalFilmDecoder();
  const provisional = decoder.push(bytes);
  assert.equal(provisional.status, 'provisional');
  assert.deepEqual(provisional.records, [{
    path: 'not-a-path',
    kind: 'StringLiteral',
    value: 'x',
  }]);
  assert.throws(
    () => decoder.finish(),
    (error) => error instanceof FilmDecodeError
      && error.stage === 'aes'
      && error.diagnostics.some(({ code }) => code === 'AES_INVALID_PATH'),
  );
});

test('does not let provisional result mutation alter staged completion state', () => {
  const bytes = fromHex('4f5f5fff010012000109242e6d6573736167650568656c6c6f');
  const decoder = new IncrementalFilmDecoder();
  const provisional = decoder.push(bytes);
  provisional.records[0].value = 'changed';
  provisional.stream.records[0].path = '$.changed';
  const completed = decoder.finish();
  assert.equal(completed.stream.records[0].path, '$.message');
  assert.equal(completed.stream.records[0].value, 'hello');
});

test('releases a fully consumed caller buffer between incremental calls', () => {
  const bytes = fromHex('4f5f5fff010012000109242e6d6573736167650568656c6c6f');
  const decoder = new IncrementalFilmDecoder();
  const provisional = decoder.push(bytes);
  assert.equal(provisional.status, 'provisional');
  assert.equal(decoder.buffer.byteLength, 0);
  assert.equal(decoder.buffer.buffer.byteLength, 0);
  assert.equal(decoder.finish().status, 'complete');
});

test('turns unfinished outer framing into truncation only when input is final', () => {
  const decoder = new IncrementalFilmDecoder();
  assert.equal(decoder.push(fromHex('4f5f5fff')).status, 'need-more-input');
  assert.throws(
    () => decoder.finish(),
    (error) => error instanceof FilmDecodeError && error.code === 'FILM_TRUNCATED',
  );
  assert.throws(() => decoder.finish(), /Film decoder is already closed/u);
});

test('rejects malformed content inside a complete declared record immediately', () => {
  const invalidKind = fromHex('4f5f5fff010008000003242e610178');
  assert.throws(
    () => new IncrementalFilmDecoder().push(invalidKind),
    (error) => error instanceof FilmDecodeError && error.code === 'FILM_INVALID_KIND',
  );
});

test('bounds bytes retained between incremental calls', () => {
  assert.throws(
    () => new IncrementalFilmDecoder({ limits: { maxBufferedBytes: 2 } })
      .push(fromHex('4f5f5f')),
    (error) => error instanceof FilmDecodeError && error.code === 'FILM_LIMIT_EXCEEDED',
  );
});

test('processes large caller chunks without an over-limit concatenation', () => {
  const bytes = fromHex('4f5f5fff010012000109242e6d6573736167650568656c6c6f');
  const decoder = new IncrementalFilmDecoder({ limits: { maxBufferedBytes: 18 } });
  assert.equal(decoder.push(bytes.subarray(0, 1)).status, 'need-more-input');
  const completed = decoder.push(bytes.subarray(1), { final: true });
  assert.equal(completed.status, 'complete');
  assert.equal(completed.stream.records[0].value, 'hello');
});

test('deterministic arbitrary-byte fuzzing produces only accepted streams or Film errors', () => {
  const random = xorshift32(0x46_49_4c_4d);
  for (let iteration = 0; iteration < 4_000; iteration += 1) {
    const bytes = randomBytes(random, random() % 257);
    assertFilmOutcome(() => decodeFilmSyntax(bytes), `one-shot iteration ${iteration}`);

    const decoder = new IncrementalFilmDecoder();
    let offset = 0;
    let failed = false;
    let completed;
    while (offset < bytes.byteLength) {
      const width = Math.min(1 + (random() % 23), bytes.byteLength - offset);
      const final = offset + width === bytes.byteLength;
      try {
        completed = decoder.push(bytes.subarray(offset, offset + width), { final });
      } catch (error) {
        assert.ok(error instanceof FilmDecodeError, `incremental iteration ${iteration}: ${error}`);
        failed = true;
        break;
      }
      offset += width;
    }
    if (!failed && bytes.byteLength !== 0) {
      assert.equal(completed.status, 'complete', `incremental iteration ${iteration}`);
    }
    if (!failed && bytes.byteLength === 0) {
      assertFilmOutcome(() => decoder.finish(), `incremental empty iteration ${iteration}`);
    }
  }
});

test('deterministic truncation and bit-flip fuzzing covers nested descriptor boundaries', () => {
  const fixture = fixtures.find(({ id }) => id === 'film-record-005-complete-optional-fields');
  assert.notEqual(fixture, undefined);
  const canonical = fromHex(fixture.hex);
  const random = xorshift32(0x44_54_59_50);
  for (let iteration = 0; iteration < 2_000; iteration += 1) {
    const length = random() % (canonical.byteLength + 1);
    const candidate = canonical.slice(0, length);
    if (candidate.byteLength !== 0 && (random() & 1) === 1) {
      const index = random() % candidate.byteLength;
      candidate[index] ^= 1 << (random() % 8);
    }
    assertFilmOutcome(
      () => decodeFilm(candidate, fixture.options),
      `descriptor mutation ${iteration}`,
    );
  }
});

function positiveFilmFixtures() {
  const output = [];
  const seen = new Set();
  const manifest = readJson(manifestUrl);
  for (const suiteRef of manifest.suites) {
    const suite = readJson(new URL(suiteRef.file, manifestUrl));
    for (const vector of suite.tests) {
      const hex = vector.operation === 'decode' && vector.expected.ok === true
        ? vector.input.film_hex
        : vector.expected.film_hex;
      if (hex === undefined) continue;
      const options = vectorOptions(vector);
      const key = JSON.stringify([hex, options]);
      if (seen.has(key)) continue;
      seen.add(key);
      output.push({ id: vector.id, hex, options });
    }
  }
  return output;
}

function vectorOptions(vector) {
  return {
    registeredFields: vector.input.registered_fields ?? [],
    limits: Object.fromEntries(Object.entries(vector.input.limits ?? {}).map(([name, value]) => [
      LIMIT_NAMES[name] ?? name,
      value,
    ])),
  };
}

function assertFilmOutcome(operation, label) {
  try {
    operation();
  } catch (error) {
    assert.ok(error instanceof FilmDecodeError, `${label}: ${error?.stack ?? error}`);
  }
}

function xorshift32(seed) {
  let state = seed >>> 0;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return state >>> 0;
  };
}

function randomBytes(random, length) {
  return Uint8Array.from({ length }, () => random() & 0xff);
}

function fromHex(input) {
  return Uint8Array.from(input.match(/../gu) ?? [], (pair) => Number.parseInt(pair, 16));
}

function readJson(url) {
  return JSON.parse(readFileSync(url, 'utf8'));
}
