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

The type-only generic correction uses `aes-events-cts-v1-snapshot-0.2`,
`telex-cts-v1-snapshot-0.2`, and `film-cts-v1-snapshot-0.3`. The current Film
claim advances to `film-cts-v1-snapshot-0.4` for shared AES logical generic-depth
accounting. Superseded numeric-generic and physical-depth expectations remain
intact in historical snapshots. Neither correction reassigns Film tags or
changes either wire framing version.

## Reference implementation versions

The repository-root npm manifest remains private at `0.0.0`; that value
identifies local JavaScript reference tooling rather than a released package.

The Rust implementation is published as `altopelago-aes-telex` on an
independent SemVer line. Version `0.2.x` implements the historical `aes.events.v1` and
`telex.aes=1`, and carries explicit CTS snapshot claims in
`conformance/cts-claims.json`. Its package version does not imply or replace an
AES event-contract, Telex wire-format, Film wire-format, or CTS snapshot
version.

Published 0.3.0 adopts the type-only generic correction and AES/Telex 0.2 and
Film 0.3 snapshot claims. Its artifacts and historical acceptance remain fixed.

The prepared 0.3.1 package corrects Film logical-depth accounting and adopts
Film CTS 0.4, retaining AES/Telex 0.2. This is a patch-only correctness fix:
it adds no public API or wire feature and does not tighten logical AES
acceptance. Preparation is not publication, and source CTS claims do not
change previously published artifacts or permit republishing their version.

Breaking Rust API changes advance the crate's package version according to
SemVer. Incompatible contract or encoding changes independently require the
new contract or wire-format version described above, even when the Rust API
could represent the change without a SemVer-major release.
