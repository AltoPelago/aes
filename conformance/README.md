# Telex conformance vectors

The repository-local Draft 0 vectors are rooted at:

```text
telex/v0/telex-cts.v0.json
```

JSON is only the language-neutral manifest envelope. Each `input.telex` value
contains the actual Telex representation under test.

The manifest separates:

- syntax and canonicalization;
- event-local AES validation; and
- complete versus partial stream validation; and
- explicit AEON document projection and header-plane validation; and
- optional record-local origin and span validation; and
- WTC anchor/reference preservation and consumer-authority boundaries.

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

These vectors remain development snapshots while the format is Draft 0. The
JavaScript and Rust implementations both consume the same vectors independently.
After the specification stabilizes, promote an immutable snapshot into the
shared `aeonite-cts` repository and rerun both implementations against that
frozen snapshot before declaring Draft 1.

The Telex vectors test portable record and event-profile behavior. They do not
assert byte-for-byte reconstruction of an originating AEON document; exact
source fidelity requires the separately retained artifact identified by
provenance.
