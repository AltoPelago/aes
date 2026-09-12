import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const manifestUrl = new URL('../conformance/film/v1/film-cts.v1.json', import.meta.url);
const outputDirectory = process.argv[2] === undefined
  ? fileURLToPath(new URL('../implementations/rust/fuzz/corpus/film_decode/', import.meta.url))
  : process.argv[2];
const manifest = readJson(manifestUrl);
const fixtures = new Map();

for (const suiteRef of manifest.suites) {
  const suite = readJson(new URL(suiteRef.file, manifestUrl));
  for (const vector of suite.tests) {
    for (const hex of [vector.input.film_hex, vector.expected.film_hex]) {
      if (hex === undefined || fixtures.has(hex)) continue;
      fixtures.set(hex, vector.id);
    }
  }
}

mkdirSync(outputDirectory, { recursive: true });
for (const [hex, id] of fixtures) {
  writeFileSync(`${outputDirectory}/${id}.film.aes`, Buffer.from(hex, 'hex'));
}
console.log(`Exported ${fixtures.size} Film fuzz seeds to ${outputDirectory}`);

function readJson(url) {
  return JSON.parse(readFileSync(url, 'utf8'));
}
