# Changelog

Notable changes to the public AES reference repository are documented here.
Contract versions, CTS snapshots, and implementation/package versions remain
independent as described in [VERSIONING.md](./VERSIONING.md).

## Unreleased

### Breaking

- Prepare `altopelago-aes-telex` 0.3.0 for type-only generic arguments. AES
  validators, Telex parsers and writers, and validated Film readers/writers
  reject literal generics, including nested occurrences. Put numeric
  parameters in clarifiers, for example `custom[2]` instead of a bare `2`.
  Exact numeric clarifier strings are unchanged.
- Adopt `aes-events-cts-v1-snapshot-0.2`, `telex-cts-v1-snapshot-0.2`, and
  `film-cts-v1-snapshot-0.3`. Historical CTS snapshots retain their old
  expectations; these sources no longer claim the superseded Telex/Film
  targets. The wire versions, Film tags, and assigned kind codes are unchanged.

### Compatibility

- Historical Film generic tag `02` remains recognizable during provisional
  physical decoding, but never represents a valid completed AES record.
  Rust retains `GenericArgument::NumberLiteral` for that unvalidated data;
  validated encoders reject it before emitting bytes. The structural Rust
  descriptor renderer is not an acceptance or validation API.
- These source changes do not publish a crate or enable durable Film writers.
  The Film depth correction requires a separately versioned provider release
  before downstream registry consumers adopt it; existing published artifacts
  remain unchanged.

### Fixed

- Align Film reader and writer generic-depth accounting with AES and Telex:
  only generic-bearing descriptor levels contribute, so `list<int>` fits a
  maximum of zero. Readers reject over-limit non-leaf descriptors after their
  bounded name/count inspection and before generic allocation or descent.
  Independent byte, argument, clarifier, component and malformed-input guards
  remain in force.
- Advance the current Film claim from snapshot 0.3 to immutable
  `film-cts-v1-snapshot-0.4` (128 operations), preserving historical snapshots.
  Cover logical depth boundaries, siblings, clarified leaves, early rejection,
  incremental split failures and borrowed/owned Rust diagnostic parity.

## 0.2.0 - 2026-10-02

### Added

- Added `SymbolicLiteral` as a distinct value-bearing AES kind with decoded
  string payload transport in Telex and typed JavaScript/Rust event APIs.
- Added the `film.aes` v1 normative draft, table-free Rust Candidate A
  prototype, stateful address-compression comparator, safety vectors, and
  reproducible layout benchmarks while retaining the durable-writer gate.
- Added a mutable Film v1 CTS, initially with 68 vectors, for framing,
  canonical bytes, all kind codes, record features, Telex equivalence,
  truncation, diagnostics, and Film-local and shared AES resource limits, with
  a Rust candidate harness.
- Added an independent JavaScript Film v1 decoder with provisional physical and
  validated owned-result surfaces. It passes all 68 direct decode vectors and
  the three canonical Film producer fixtures without invoking Rust or parsing
  Telex in the decoder.
- Established the public AES repository authority, governance, contribution,
  security, versioning, and release boundaries.
- Added independently testable JavaScript and Rust reference implementations
  for `telex.aes=1` and `aes.events.v1`.
- Added mutable local development vectors plus runners for the immutable
  `aes-events-cts-v1-snapshot-0.1`, `telex-cts-v1-snapshot-0.1`, and
  `film-cts-v1-snapshot-0.1` targets.
- Added repository CI, dependency review, Rust advisory scanning, Dependabot,
  issue templates, and local-path hygiene checks.

### Changed

- Advanced the symbol-capable Film contract and shared conformance target to
  immutable snapshot `0.2`, preserving snapshot `0.1` unchanged.
- Released the Rust reference crate as `altopelago-aes-telex` `0.2.0` for
  coordinated symbol-capable AEON builds.
- Appended the Film mapping for `SymbolicLiteral` at kind code `0x18`, leaving
  all existing Film kind codes unchanged.
- Replaced the experimental Candidate B decode wrapper with a direct general
  decoder covering all Film fields, limits, prefix canonicality, diagnostics,
  and complete AES validation; retained the wrapper as an explicit benchmark
  baseline and added Candidate B to the bounded fuzz target.
- Added experimental checkpointed Film Candidate C with mandatory 256-record
  absolute addresses, checkpoint indexing and block recovery, a general compact
  typed owned result, native portable-AES validation without `TelexRecord`
  materialization, an opt-in cached-path evidence experiment, and a shared
  transient path-arena validation experiment available to both Film v1 and
  Candidate C, with diagnostic-equivalence tests, full-field round trips,
  resource bounds, and fuzz coverage. Its direct
  decoder reuses the active body and header address buffers during prefix
  reconstruction instead of allocating one buffer per record.
- Promoted the audited Film v1 wire contract and 72-vector candidate to
  immutable specification and CTS snapshot `0.1` authorities. JavaScript now
  records a scoped `decode` claim, while Rust claims all Film operations.
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
- Kept the repository-root npm reference package private while publishing the
  Rust implementation through a signed-tag, protected-environment, crates.io
  trusted-publisher workflow.
