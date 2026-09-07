# Telex conformance vectors

The repository-local Draft 0 vectors are rooted at:

```text
telex/v0/telex-cts.v0.json
```

JSON is only the language-neutral manifest envelope. Each `input.telex` value
contains the actual Telex representation under test. The manifest and each
suite bind those vectors to portable contract `aes.events.v0`; `format_version`
continues to identify Telex encoding version `0` independently.

An optional `input.limits` object supplies normalized effective integer limits
to the harness using the canonical snake-case counter names. It is trusted test
configuration, not Telex stream metadata, and unspecified counters retain the
published implementation defaults.

Event and profile expectations derive from
`specifications/aes.events.md`. Telex syntax and canonical-byte expectations
derive from `specifications/telex.aes.md`.

The separate mutable AES integrity candidate is rooted at:

```text
integrity/v0/aes-integrity-cts.v0.json
```

Its vectors derive from `specifications/aes.integrity.md` and compare
transport-independent logical bytes, SHA-256 digests, ordering and provenance
policies, scope, and signature-context input. It is not part of either
published snapshot 0.1 and does not change those stable external targets.

The manifest separates:

- syntax and canonicalization;
- event-local AES validation;
- complete versus partial stream validation;
- complete-profile reference-target integrity and partial dangling references;
- explicit AEON document projection and header-plane validation;
- optional record-local origin and non-empty span validation;
- WTC anchor/reference preservation and consumer-authority boundaries; and
- inclusive at-limit and rejecting one-over-limit behavior for every Telex v0
  resource counter.

Syntax vectors also distinguish canonical empty-stream EOF, tolerant Unicode
escape spelling, bare CR, raw LF framing, and exact field-name segment grammar.
They also verify compact datatype expansion, recursive generics, ordered tagged
clarifiers, duplicate preservation, and numeric payload precision.
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

These 88 integrated vectors remain the mutable development candidate. The
candidate uses a `-dev` version and carries no `snapshot_id` or
`spec_snapshot_id`. A repository commit identifies an exact development state,
but external conformance claims must not treat this working path as stable.

The first stable shared publication separates the owning contracts:

- `aes-events-cts-v0-snapshot-0.1` contains 38 transport-neutral event and
  profile-validation vectors;
- `telex-cts-v0-snapshot-0.1` contains 50 Telex syntax, canonicalization, and
  format-limit vectors.

Both manifests live in `aeonite-cts`, pin a SHA-256 digest for every suite, and
pass independently in the JavaScript and Rust implementations. Later candidate
changes stay here or enter a newer shared snapshot; the published identifiers
and their suite bytes are not changed.

The Telex vectors test portable record and event-profile behavior. They do not
assert byte-for-byte reconstruction of an originating AEON document; exact
source fidelity requires the separately retained artifact identified by
provenance.
