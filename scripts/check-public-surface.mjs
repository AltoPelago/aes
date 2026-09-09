#!/usr/bin/env node
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const errors = [];
const requiredFiles = [
  'AUTHORITY.md',
  'CHANGELOG.md',
  'CODE_OF_CONDUCT.md',
  'CONTRIBUTING.md',
  'GOVERNANCE.md',
  'LICENSE',
  'README.md',
  'RELEASING.md',
  'SECURITY.md',
  'VERSIONING.md',
  '.github/CODEOWNERS',
  '.github/dependabot.yml',
  '.github/workflows/ci.yml',
  '.github/workflows/dependency-security.yml',
  '.github/workflows/repo-hygiene.yml',
  'conformance/cts-claims.json',
];

for (const file of requiredFiles) {
  if (!existsSync(resolve(root, file))) errors.push(`missing public surface file: ${file}`);
}

const packageJson = JSON.parse(readFileSync(resolve(root, 'package.json'), 'utf8'));
if (packageJson.name !== '@altopelago/aes') errors.push('unexpected root package name');
if (packageJson.version !== '0.0.0') errors.push('root reference package version must remain 0.0.0');
if (packageJson.private !== true) errors.push('root reference package must remain private');
if (packageJson.publishConfig !== undefined) errors.push('root reference package must not define publishConfig');

const cargo = readFileSync(resolve(root, 'implementations/rust/Cargo.toml'), 'utf8');
if (!/^publish = false$/mu.test(cargo)) errors.push('Rust reference crate must remain non-publishable');

const license = readFileSync(resolve(root, 'LICENSE'), 'utf8');
if (!license.includes('Copyright (c) 2026 AltoPelago')) {
  errors.push('LICENSE must identify AltoPelago as copyright holder');
}

if (errors.length > 0) {
  console.error(`Public surface check failed: ${errors.length} issue(s)`);
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log('Public surface check passed; package publication remains disabled.');
