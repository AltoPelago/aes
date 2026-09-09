# Versioning

This repository has separate version tracks. They must not be inferred from one
another.

## AES event contracts

The portable semantic event line is currently `aes.events.v1`, with
`aes.complete.v1`, `aes.partial.v1`, and the optional `aeon.document.v1`
projection.

An incompatible event-model change requires a new AES event-contract version.

## Encoding versions

Telex has its own wire-format version. The current preamble is:

```text
telex.aes=1
```

An incompatible framing or decoding change requires a new Telex format
version. Film and future encodings will have independent wire versions while
carrying a declared AES event contract.

## Conformance snapshots

Shared CTS snapshot identifiers are immutable release targets within a contract
line. Compatible additional coverage receives a new snapshot identifier; an
existing released snapshot is never silently rewritten.

Current public baselines include:

- `aes-events-cts-v1-snapshot-0.1`
- `telex-cts-v1-snapshot-0.1`

## Reference implementation versions

The repository-root npm manifest and `aes-telex` Rust crate currently use
`0.0.0` and are explicitly private/non-publishable. Those values identify local
reference tooling, not the AES or Telex contract version.

If either implementation is later released as a package, it must receive an
independent SemVer line, a public package name, a release workflow, and a
documented compatibility declaration. Publishing a package must not reuse the
AES event or Telex wire version as its package version by implication.
