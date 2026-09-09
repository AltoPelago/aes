# Changelog

Notable changes to the public AES reference repository are documented here.
Contract versions, CTS snapshots, and implementation/package versions remain
independent as described in [VERSIONING.md](./VERSIONING.md).

## Unreleased

### Added

- Established the public AES repository authority, governance, contribution,
  security, versioning, and release boundaries.
- Added independently testable JavaScript and Rust reference implementations
  for `telex.aes=1` and `aes.events.v1`.
- Added mutable local development vectors plus runners for the immutable
  `aes-events-cts-v1-snapshot-0.1` and `telex-cts-v1-snapshot-0.1` targets.
- Added repository CI, dependency review, Rust advisory scanning, Dependabot,
  issue templates, and local-path hygiene checks.

### Changed

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
