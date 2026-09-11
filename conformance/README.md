# AES conformance vectors

The repository-local v1 vectors are rooted at:

```text
telex/v1/telex-cts.v1.json
```

The repository-local Film v1 candidate is rooted separately at:

```text
film/v1/film-cts.v1.json
```

The transport-neutral AES event candidate is rooted separately at:

```text
aes/v1/aes-events-cts.v1.json
```

It keeps portable records independent of Telex framing. Both local manifests
are mutable `-dev` targets; neither may be cited as an immutable external
snapshot.

JSON is only the language-neutral manifest envelope. Each `input.telex` value
contains the actual Telex representation under test. The manifest and each
suite bind those vectors to portable contract `aes.events.v1`; `format_version`
continues to identify Telex encoding version `1` independently.

An optional `input.limits` object supplies normalized effective integer limits
to the harness using the canonical snake-case counter names. It is trusted test
configuration, not Telex stream metadata, and unspecified counters retain the
published implementation defaults.

Event and profile expectations derive from
`specifications/aes.events.md`. Telex syntax and canonical-byte expectations
derive from `specifications/telex.aes.md`.

The separate mutable AES integrity candidate is rooted at:

```text
integrity/v1/aes-integrity-cts.v1.json
```

Its vectors derive from `specifications/aes.integrity.md` and compare
transport-independent logical bytes, SHA-256 digests, ordering and provenance
policies, scope, and signature-context input. It is not part of either
published snapshot 0.1 and does not change those stable external targets.

The separate mutable Assignment Event Transaction candidate is rooted at:

```text
transactions/v1/aes-transaction-cts.v1.json
```

It covers the transport-neutral transaction body and logical envelope,
exact-order transaction integrity, support-versus-authority gating, and the
initial narrow ASP scalar-value replacement application. It does not change
Telex v1 framing or enable bare-stream, Wire, or CLI mutation ingress.

The separate mutable exact-source provenance candidate is rooted at:

```text
provenance/v1/aes-provenance-cts.v1.json
```

It verifies record-local origins and UTF-8 byte ranges against exact retained
artifacts, distinguishes unavailable evidence from invalid evidence, and gates
the source-backed preparation contract. It is not a published immutable CTS
snapshot.

The [`film.aes` v1 specification](../specifications/film.aes.md) is a normative
draft. Its mutable `0.1.0-dev` CTS contains 68 language-neutral vectors across
framing and canonicalization, records and Telex transcoding, and resource
limits. Exact Film bytes use contiguous lowercase hexadecimal inside the JSON
test envelope. The lane fixes all 23 kind-code mappings and keeps Film syntax
and canonicality failures separate from transport-neutral AES diagnostics.

The Film candidate carries no `snapshot_id` or `spec_snapshot_id`. Passing it
does not establish an immutable external conformance claim, and the current
selected Rust reference runner is not a second independent decoder.

The manifest separates:

- syntax and canonicalization;
- event-local AES validation;
- complete versus partial stream validation;
- complete-profile reference-target integrity and partial dangling references;
- explicit AEON document projection and header-plane validation;
- optional record-local origin and non-empty span validation;
- WTC anchor/reference preservation and consumer-authority boundaries; and
- inclusive at-limit and rejecting one-over-limit behavior for every Telex v1
  format counter and every shared AES structural counter applied at Telex
  ingress.

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

Film suites additionally use:

- `decode`: accept exact `film_hex`, return logical stream context and records,
  or a stable Film error; failures after Film decoding identify the AES stage
  and retain its diagnostic codes without prescribing a host-language wrapper;
- `encode`: produce exact canonical `film_hex` from one logical stream under
  the supplied effective limits; and
- `transcode`: convert canonical Telex directly to exact Film bytes and back
  without reconstructing AEON source.

Run the mutable Film manifest checks and Rust candidate harness together with:

```bash
npm run test:film
```

The 100 integrated Telex vectors remain their mutable development candidate. The
candidate uses a `-dev` version and carries no `snapshot_id` or
`spec_snapshot_id`. A repository commit identifies an exact development state,
but external conformance claims must not treat this working path as stable.

The first stable shared publication separates the owning contracts:

- `aes-events-cts-v1-snapshot-0.1` contains 38 transport-neutral event and
  profile-validation vectors;
- `telex-cts-v1-snapshot-0.1` contains 50 Telex syntax, canonicalization, and
  format-limit vectors.

Both manifests live in `aeonite-cts`, pin a SHA-256 digest for every suite, and
pass independently in the JavaScript and Rust implementations. Later candidate
changes stay here or enter a newer shared snapshot; the published identifiers
and their suite bytes are not changed.

The repository declares its released-snapshot coverage in
[`cts-claims.json`](cts-claims.json). Validate those claims against an
`aeonite-cts` checkout with:

```bash
AEONITE_CTS_ROOT=/path/to/aeonite-cts/cts npm run validate:cts-claims
npm run test:conformance:shared
npm run test:conformance:rust:shared
```

Without `AEONITE_CTS_ROOT`, these commands look for the standard Aeonite family
sibling layout. The default `npm test` and `npm run test:rust` commands use only
the mutable vectors in this repository, so a standalone clone remains
independently testable.

The Telex vectors test portable record and event-profile behavior. They do not
assert byte-for-byte reconstruction of an originating AEON document; exact
source fidelity requires the separately retained artifact identified by
provenance.
