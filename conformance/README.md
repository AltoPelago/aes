# Telex conformance vectors

The repository-local Draft 0 vectors are rooted at:

```text
telex/v0/telex-cts.v0.json
```

JSON is only the language-neutral manifest envelope. Each `input.telex` value
contains the actual Telex representation under test. The manifest and each
suite bind those vectors to portable contract `aes.events.v0`; `format_version`
continues to identify Telex encoding version `0` independently.

Event and profile expectations derive from
`specifications/aes.events.md`. Telex syntax and canonical-byte expectations
derive from `specifications/telex.aes.md`.

The manifest separates:

- syntax and canonicalization;
- event-local AES validation; and
- complete versus partial stream validation; and
- complete-profile reference-target integrity and partial dangling references;
- explicit AEON document projection and header-plane validation; and
- optional record-local origin and non-empty span validation; and
- WTC anchor/reference preservation and consumer-authority boundaries.

Syntax vectors also distinguish canonical empty-stream EOF, tolerant Unicode
escape spelling, bare CR, raw LF framing, and exact field-name segment grammar.
Source-backed digest, byte-bound, and UTF-8 boundary audits require the external
artifact and are therefore outside the local Telex vector operations.

Portable tests assert stable error or diagnostic codes rather than matching
implementation-specific prose. Semantic `diagnostic_codes` are compared as a
multiset: diagnostic order is not part of AES conformance. Record numbers in
diagnostics are zero-based, matching event sequence indexes; syntax line
numbers are one-based for editors and command-line diagnostics.

An implementation adopting these vectors must support the operation named by
each test:

- `parse`: return version, effective profile, whether it was explicit,
  optional projection and its explicitness, canonicality, and decoded records;
- `canonicalize`: return canonical Telex text without reordering events; or
- `validate`: return the effective profile and semantic diagnostic codes.

These vectors are a mutable development candidate while the format is Draft 0.
The candidate uses a `-dev` version and carries no `snapshot_id` or
`spec_snapshot_id`. A repository commit identifies an exact development state,
but external conformance claims must not treat this working path as stable.

The JavaScript and Rust implementations consume the candidate independently.
After the specification stabilizes, the release process copies the exact suite
and specifications into immutable artifacts in the shared `aeonite-cts`
repository, mints their snapshot identifiers once, records content digests, and
tags the release. Published snapshot identifiers are never reused or moved to
different content. Both implementations must pass that frozen snapshot before
Draft 1 is declared.

The Telex vectors test portable record and event-profile behavior. They do not
assert byte-for-byte reconstruction of an originating AEON document; exact
source fidelity requires the separately retained artifact identified by
provenance.
