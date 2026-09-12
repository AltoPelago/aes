import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import { FilmDecodeError, decodeFilm } from '../src/film.js';
import { parseTelex } from '../src/telex.js';

const manifestUrl = new URL('../conformance/film/v1/film-cts.v1.json', import.meta.url);
const manifest = readJson(manifestUrl);
let directDecodeVectors = 0;
let producerFixtures = 0;
let encoderOnlyVectors = 0;

for (const suiteRef of manifest.suites) {
  const suite = readJson(new URL(suiteRef.file, manifestUrl));
  for (const vector of suite.tests) {
    if (vector.operation === 'decode') {
      directDecodeVectors += 1;
      test(`javascript-decoder/${vector.id}`, () => runDecodeVector(vector));
    } else if (vector.expected.film_hex !== undefined) {
      producerFixtures += 1;
      test(`javascript-decoder/${vector.id}`, () => runProducerFixture(vector));
    } else {
      encoderOnlyVectors += 1;
    }
  }
}

test('independent JavaScript decoder coverage has an explicit decoder-only boundary', () => {
  assert.deepEqual({ directDecodeVectors, producerFixtures, encoderOnlyVectors }, {
    directDecodeVectors: 68,
    producerFixtures: 3,
    encoderOnlyVectors: 1,
  });
});

function runDecodeVector(vector) {
  try {
    const stream = decodeFilm(fromHex(vector.input.film_hex), vectorOptions(vector));
    assert.notEqual(vector.expected.ok, false, 'expected decoding to fail');
    if (vector.expected.stream !== undefined) {
      assert.deepEqual(streamForCts(stream), vector.expected.stream);
    }
  } catch (error) {
    assert.equal(vector.expected.ok, false, `unexpected decode error: ${error.message}`);
    assert.ok(error instanceof FilmDecodeError);
    if (vector.expected.error.code !== undefined) {
      assert.equal(error.code, vector.expected.error.code);
    } else {
      assert.equal(vector.expected.error.stage, 'aes');
      assert.equal(error.stage, 'aes');
      assert.deepEqual(
        error.diagnostics.map(({ code }) => code).sort(),
        [...vector.expected.error.diagnostic_codes].sort(),
      );
    }
  }
}

function runProducerFixture(vector) {
  const stream = decodeFilm(fromHex(vector.expected.film_hex), vectorOptions(vector));
  if (vector.operation === 'transcode') {
    const parsed = parseTelex(vector.input.telex);
    assert.deepEqual(streamForCts(stream), {
      profile: parsed.profile,
      profile_explicit: parsed.profileExplicit,
      projection: parsed.projection,
      projection_explicit: parsed.projectionExplicit,
      records: parsed.records,
    });
    return;
  }
  assert.equal(vector.operation, 'encode');
  assert.deepEqual(streamForCts(stream), vector.input.stream);
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

function streamForCts(stream) {
  return {
    profile: stream.profile,
    profile_explicit: stream.profileExplicit,
    projection: stream.projection,
    projection_explicit: stream.projectionExplicit,
    records: stream.records,
  };
}

function fromHex(input) {
  return Uint8Array.from(input.match(/../gu) ?? [], (pair) => Number.parseInt(pair, 16));
}

function readJson(url) {
  return JSON.parse(readFileSync(url, 'utf8'));
}

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
