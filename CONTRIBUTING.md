# Contributing

AES is public and readable, but this repository is not run as a standard
open-contribution project.

The governance model is described in [GOVERNANCE.md](./GOVERNANCE.md). The short
version is:

- reference implementation authority lives in this repository
- specification and shared conformance authority live in their named sibling repositories
- release and merge authority stay with the maintainer
- outside pull requests are not assumed to be part of the default workflow

## What is welcome

- clear bug reports
- reproducible Telex parsing, encoding, or canonicalization mismatches
- portable event-model and cross-implementation parity findings
- CTS gaps and interoperability reports
- documentation fixes
- forks, downstream tooling, and independent implementations

## Before opening a pull request

- do not assume an unsolicited pull request will be merged
- prefer opening an issue first for non-trivial changes
- identify whether the change affects a specification, immutable CTS target,
  reference implementation, or proposal
- include a concrete failing input and expected result for behavioral changes
- keep changes narrowly scoped and avoid bundling unrelated cleanup
- run `npm run precommit` so tracked files do not introduce machine-local paths

## Development expectations

- run `npm test` for the self-contained JavaScript and local conformance suite
- run `npm run test:rust` for the independent Rust implementation
- run the shared CTS commands when changing published AES or Telex behavior
- run `npm run check:rust` when changing Rust code
- keep canonical bytes and diagnostics deterministic across implementations
- do not modify an immutable shared CTS snapshot in place

Shared CTS commands require an `aeonite-cts` checkout. Set
`AEONITE_CTS_ROOT` to its `cts/` directory, or use the standard sibling layout
described in [README.md](./README.md).

## Good contribution shape

- a minimal reproduction
- the affected contract and authority surface
- focused implementation changes
- tests or development vectors that lock the behavior in
- an explicit migration note when canonical bytes or identifiers change

## Security issues

Do not use normal public issues for suspected vulnerabilities. Follow
[SECURITY.md](./SECURITY.md) instead.
