#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');
const ctsRoot = resolve(
  readArg('--cts-root')
    ?? process.env.AEONITE_CTS_ROOT
    ?? resolve(root, '..', '..', 'aeonite-org', 'aeonite-cts', 'cts'),
);
const claimsPath = resolve(
  readArg('--claims') ?? resolve(root, 'conformance', 'cts-claims.json'),
);
const errors = [];

if (!existsSync(claimsPath)) fail(`claims file does not exist: ${claimsPath}`);
if (!existsSync(ctsRoot)) {
  fail(
    `CTS root does not exist: ${ctsRoot}\n`
    + 'Set AEONITE_CTS_ROOT to the cts/ directory of an aeonite-cts checkout.',
  );
}

const document = readJson(claimsPath, 'claims file');
const snapshotIds = readCtsSnapshotIds(ctsRoot);
const claims = collectClaims(document);

if (document.claim_format !== 'aeonite.cts-claims.v1') {
  errors.push('claim_format must be "aeonite.cts-claims.v1"');
}
if (document.repository !== 'AltoPelago/aes') {
  errors.push('repository must be "AltoPelago/aes"');
}
if (document.cts_repository !== 'aeonite-org/aeonite-cts') {
  errors.push('cts_repository must be "aeonite-org/aeonite-cts"');
}
if (document.cts_protocol !== 'cts.protocol.v1') {
  errors.push('cts_protocol must be "cts.protocol.v1"');
}

claims.forEach((claim) => validateClaim(claim, snapshotIds));

if (errors.length > 0) {
  console.error(`CTS claim validation failed: ${errors.length} issue(s)`);
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log(
  `CTS claim validation passed: claims=${claims.length} cts_snapshots=${snapshotIds.size}`,
);

function readArg(name) {
  const index = process.argv.indexOf(name);
  if (index < 0) return null;
  const value = process.argv[index + 1];
  if (value === undefined || value.startsWith('--')) return null;
  return value;
}

function readJson(file, label) {
  try {
    return JSON.parse(readFileSync(file, 'utf8'));
  } catch (error) {
    fail(`invalid JSON in ${label}: ${file}: ${error.message}`);
  }
}

function collectClaims(value) {
  if (!Array.isArray(value.claim_sets)) {
    errors.push('claims document must contain claim_sets');
    return [];
  }
  return value.claim_sets.flatMap((claimSet, claimSetIndex) => {
    const path = `claim_sets[${claimSetIndex}]`;
    if (typeof claimSet?.implementation !== 'string' || claimSet.implementation.length === 0) {
      errors.push(`${path}.implementation must be a non-empty string`);
    }
    if (typeof claimSet?.implementation_version !== 'string'
      || claimSet.implementation_version.length === 0) {
      errors.push(`${path}.implementation_version must be a non-empty string`);
    }
    if (!Array.isArray(claimSet?.claims)) {
      errors.push(`${path}.claims must be a list`);
      return [];
    }
    return claimSet.claims.map((claim, claimIndex) => ({
      claim,
      path: `${path}.claims[${claimIndex}]`,
    }));
  });
}

function validateClaim({ claim, path }, knownSnapshotIds) {
  if (!claim || typeof claim !== 'object') {
    errors.push(`${path} must be an object`);
    return;
  }
  if (typeof claim.surface !== 'string' || claim.surface.length === 0) {
    errors.push(`${path}.surface must be a non-empty string`);
  }
  if (typeof claim.snapshot_id !== 'string' || claim.snapshot_id.length === 0) {
    errors.push(`${path}.snapshot_id must be a non-empty string`);
  } else if (!knownSnapshotIds.has(claim.snapshot_id)) {
    errors.push(`${path}.snapshot_id is not present in CTS manifests: ${claim.snapshot_id}`);
  }
  if (claim.status !== 'claimed') {
    errors.push(`${path}.status must be "claimed" for a released public baseline`);
  }
  if (typeof claim.command !== 'string' || claim.command.length === 0) {
    errors.push(`${path}.command must be a non-empty string`);
  }
}

function readCtsSnapshotIds(directory) {
  const ids = new Set();
  for (const file of walkJsonFiles(directory)) {
    const manifest = readJson(file, 'CTS manifest');
    if (Array.isArray(manifest.suites) && typeof manifest.meta?.snapshot_id === 'string') {
      ids.add(manifest.meta.snapshot_id);
    }
  }
  return ids;
}

function walkJsonFiles(directory) {
  const output = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) output.push(...walkJsonFiles(path));
    else if (entry.isFile() && entry.name.endsWith('.json')) output.push(path);
  }
  return output;
}

function fail(message) {
  console.error(`CTS claim validation failed: ${message}`);
  process.exit(1);
}
