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
version.

Film has an independent draft wire-format version. Its proposed preamble is:

```text
4F 5F 5F FF 01
```

Film v1 maps statically to `aes.events.v1`. It is not a released encoding line
until its specification and CTS snapshots are published. An incompatible Film
framing or decoding change advances the Film format version rather than the
Telex version or AES event contract by implication.

## Conformance snapshots

Shared CTS snapshot identifiers are immutable release targets within a contract
line. Compatible additional coverage receives a new snapshot identifier; an
existing released snapshot is never silently rewritten.

Current public baselines include:

- `aes-events-cts-v1-snapshot-0.1`
- `telex-cts-v1-snapshot-0.1`

## Reference implementation versions

The repository-root npm manifest remains private at `0.0.0`; that value
identifies local JavaScript reference tooling rather than a released package.

The Rust implementation is published as `altopelago-aes-telex` on an
independent SemVer line. Version `0.2.x` implements `aes.events.v1` and
`telex.aes=1`, and carries explicit CTS snapshot claims in
`conformance/cts-claims.json`. Its package version does not imply or replace an
AES event-contract, Telex wire-format, Film wire-format, or CTS snapshot
version.

Breaking Rust API changes advance the crate's package version according to
SemVer. Incompatible contract or encoding changes independently require the
new contract or wire-format version described above, even when the Rust API
could represent the change without a SemVer-major release.
