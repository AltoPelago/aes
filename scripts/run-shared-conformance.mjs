#!/usr/bin/env node
import { existsSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');
const mode = process.argv[2];
const ctsRoot = resolve(
  readArg('--cts-root')
    ?? process.env.AEONITE_CTS_ROOT
    ?? resolve(root, '..', '..', 'aeonite-org', 'aeonite-cts', 'cts'),
);

if (!['javascript', 'rust'].includes(mode)) {
  fail('usage: run-shared-conformance.mjs <javascript|rust> [--cts-root PATH]');
}

const telexManifest = resolve(
  ctsRoot,
  'telex/v1/telex-cts.v1.snapshot-0.1.json',
);
const aesEventsManifest = resolve(
  ctsRoot,
  'aes/v1/aes-events-cts.v1.snapshot-0.1.json',
);
const filmManifest = resolve(
  ctsRoot,
  'film/v1/film-cts.v1.snapshot-0.1.json',
);

for (const manifest of [telexManifest, aesEventsManifest, filmManifest]) {
  if (!existsSync(manifest)) {
    fail(
      `shared CTS manifest not found: ${manifest}\n`
      + 'Set AEONITE_CTS_ROOT to the cts/ directory of an aeonite-cts checkout.',
    );
  }
}

const environment = {
  ...process.env,
  TELEX_CTS_MANIFEST: telexManifest,
  AES_EVENTS_CTS_MANIFEST: aesEventsManifest,
  FILM_CTS_MANIFEST: filmManifest,
};
const command = mode === 'javascript' ? process.execPath : 'cargo';
const args = mode === 'javascript'
  ? [
    '--test',
    'test/conformance.test.js',
    'test/aes-events-conformance.test.js',
    'test/film-conformance.test.js',
    'test/film-decoder-conformance.test.js',
  ]
  : ['test', '--locked', '--manifest-path', 'implementations/rust/Cargo.toml'];
const result = spawnSync(command, args, {
  cwd: root,
  env: environment,
  stdio: 'inherit',
});

if (result.error !== undefined) fail(result.error.message);
process.exit(result.status ?? 1);

function readArg(name) {
  const index = process.argv.indexOf(name);
  if (index < 0) return null;
  const value = process.argv[index + 1];
  if (value === undefined || value.startsWith('--')) return null;
  return value;
}

function fail(message) {
  console.error(`Shared conformance failed: ${message}`);
  process.exit(1);
}
