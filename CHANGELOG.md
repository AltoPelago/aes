# Changelog

Notable changes to the public AES reference repository are documented here.
Contract versions, CTS snapshots, and implementation/package versions remain
independent as described in [VERSIONING.md](./VERSIONING.md).

## Unreleased

### Added

- Added the `film.aes` v1 normative draft, table-free Rust Candidate A
  prototype, stateful address-compression comparator, safety vectors, and
  reproducible layout benchmarks without declaring a released Film target.
- Added a mutable 68-vector Film v1 CTS for framing, canonical bytes, all kind
  codes, record features, Telex equivalence, truncation, diagnostics, and
  Film-local and shared AES resource limits, with a Rust candidate harness.
- Added an independent JavaScript Film v1 decoder with provisional physical and
  validated owned-result surfaces. It passes all 64 direct decode vectors and
  the three canonical Film producer fixtures without invoking Rust or parsing
  Telex in the decoder.
- Established the public AES repository authority, governance, contribution,
  security, versioning, and release boundaries.
- Added independently testable JavaScript and Rust reference implementations
  for `telex.aes=1` and `aes.events.v1`.
- Added mutable local development vectors plus runners for the immutable
  `aes-events-cts-v1-snapshot-0.1` and `telex-cts-v1-snapshot-0.1` targets.
- Added repository CI, dependency review, Rust advisory scanning, Dependabot,
  issue templates, and local-path hygiene checks.

### Changed

- Expanded the mutable Film CTS to 72 vectors and fixed cross-host precedence
  so active byte and AES count limits are applied before host-size conversion;
  incremental decoding now also releases fully consumed caller buffers.
- Added explicit incremental JavaScript Film states for incomplete input,
  provisional records, and final validated completion; added exhaustive
  positive-fixture chunk divisions, deterministic malformed-input regression
  fuzzing, and a CTS-seeded Rust libFuzzer/AddressSanitizer target with a
  scheduled workflow.
- Refactored the selected Rust Film draft implementation behind
  specification-shaped encode, borrowed-decode, validated owned-decode, and
  Telex-transcoding APIs; retained Candidate A names as temporary local
  compatibility aliases.
- Promoted the pre-publication AES and Telex identifiers to their v1 contract
  lines across reference implementations, examples, policies, and working
  documentation.
- Made the default test suites independent of the surrounding local repository
  layout while retaining explicit shared-CTS verification.

### Security

- Kept resource-limit enforcement, provenance verification, canonical logical
  bytes, and transaction inspection in the public regression surface.
- Kept both reference packages non-publishable pending an explicit package
  release decision.
