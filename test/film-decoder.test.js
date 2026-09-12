import assert from 'node:assert/strict';
import test from 'node:test';

import {
  DEFAULT_FILM_LIMITS,
  FILM_V1_PREAMBLE,
  FilmDecodeError,
  decodeFilm,
  decodeFilmSyntax,
  filmV1IsDraft,
  normalizeFilmLimits,
} from '../src/film.js';

test('publishes the Film v1 identity and proposed local limits', () => {
  assert.deepEqual(FILM_V1_PREAMBLE, [0x4f, 0x5f, 0x5f, 0xff, 0x01]);
  assert.deepEqual(DEFAULT_FILM_LIMITS, {
    maxInputBytes: 67_108_864,
    maxRecordBytes: 16_777_216,
    maxFieldBytes: 4_194_304,
    maxBufferedBytes: 16_777_216,
  });
  assert.equal(filmV1IsDraft(), true);
  assert.deepEqual(normalizeFilmLimits({ limits: { maxRecordBytes: 12 } }), {
    ...DEFAULT_FILM_LIMITS,
    maxRecordBytes: 12,
  });
});

test('keeps Film syntax decoding provisional until AES validation', () => {
  const bytes = fromHex('4f5f5fff01000f00010a6e6f742d612d706174680178');
  assert.deepEqual(decodeFilmSyntax(bytes).records, [{
    path: 'not-a-path',
    kind: 'StringLiteral',
    value: 'x',
  }]);
  assert.throws(
    () => decodeFilm(bytes),
    (error) => error instanceof FilmDecodeError
      && error.code === 'FILM_AES_INVALID'
      && error.stage === 'aes'
      && error.diagnostics.some(({ code }) => code === 'AES_INVALID_PATH'),
  );
});

test('returns owned JavaScript values independent of later byte mutation', () => {
  const storage = new Uint8Array(40);
  const encoded = fromHex('4f5f5fff010012000109242e6d6573736167650568656c6c6f');
  storage.set(encoded, 7);
  const view = storage.subarray(7, 7 + encoded.length);
  const stream = decodeFilm(view);
  storage.fill(0);
  assert.equal(stream.records[0].path, '$.message');
  assert.equal(stream.records[0].value, 'hello');
});

test('requires an exact Uint8Array byte view', () => {
  assert.throws(
    () => decodeFilm(new ArrayBuffer(6)),
    /Film input must be a Uint8Array/u,
  );
});

function fromHex(input) {
  return Uint8Array.from(input.match(/../gu) ?? [], (pair) => Number.parseInt(pair, 16));
}
