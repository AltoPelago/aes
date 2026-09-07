# Cross-repository AES/Telex acceptance record

Date: 2026-09-07

## Result

The existing AES/Telex implementation and consumer acceptance lanes pass. One
new ASP integration vector closes the previously unrepresented supported path:

```text
AEON source
  -> portable aes.events.v0 encoded as telex.aes=0
  -> named SO scalar-replacement application
  -> ASP v0 / durable AES-DB
  -> strict portable aes.events.v0 view
  -> canonical telex.aes=0
```

The fixture derives the mutation address and expected kind from the source
Telex record, verifies the initial AES-DB Telex view is equivalent, performs the
already-supported named scalar mutation, and verifies that path, identity,
datatype, and record order survive the returned Telex stream. It does not add a
Telex writer or make a bare event stream actionable.

The existing node-head negative vector remains authoritative: direct node-head
replacement returns `AES_COMPAT_NODE_HEAD_STORAGE_REQUIRED`, writes nothing,
and leaves the journal empty. This is an accepted fail-closed result, not an
acceptance gap.

## Executed lanes

| Repository/lane | Result |
| --- | --- |
| AltoPelago AES JavaScript, shared immutable AES Events and Telex CTS | 93 tests passed, including all 38 AES Events and 50 Telex vectors |
| AltoPelago AES Rust, shared immutable AES Events and Telex CTS | all conformance, completeness, and limits tests passed |
| AEON TypeScript AES Events CTS | 38 vectors passed |
| AEON TypeScript Telex CTS | 50 vectors passed |
| AEON Python Telex/AES tests under the required bundled Python runtime | 12 tests passed |
| AEON PHP AES suite | 91 tests and 256 assertions passed |
| SANSA full suite | 257 tests passed |
| ASP full suite, including the new source-derived round trip | 1,106 tests passed |
| AEON TypeScript integrity package | 14 tests passed |
| AEON TypeScript CLI | 96 tests passed |
| AEON Rust CLI | 123 tests passed; `cargo fmt --check` passed |
| Aeon Tonics focused legacy-boundary tests | `aes-diff` 30 tests and `aeon-edit` 80 tests passed |

The first sandboxed ASP full-suite run could not bind loopback test servers and
reported eight `listen EPERM` failures. The same unmodified suite was rerun with
loopback access and passed all 1,106 tests; these were environment restrictions,
not implementation failures.

## Coverage confirmed by the existing lanes

- complete versus partial profile behavior, ancestry, duplicate addresses, and
  event-local validity;
- expanded node-head paths, node/reference translation, and separation of
  structural identity from path identity;
- flattened attribute ancestry and quoted attribute paths;
- expanded datatype structure and compact Telex descriptor canonicalization;
- WTC lexical preservation, anchor/reference forms, and authority separation;
- header-plane opt-in, header-before-body ordering, and header/body address
  separation;
- origin/span dependency and canonical origin validation;
- record-order preservation and format/resource-limit boundaries;
- SANSA selection/navigation, scalar Telex mutation, identity preservation, and
  rejection of ambiguous or partial portable mutations;
- ASP strict read views, Telex export, historical reads, replay, checkpoint,
  compaction floor, backup, restore, and point-in-time recovery;
- supported scalar and contained-scalar SO dispatch plus stable rejection of
  node-head storage expansion.

## Still-open work (not failures of this run)

The more exhaustive matrix in the primary checklist remains useful future CTS
work: all structural-identity locations across every source implementation,
additional Unicode span fixtures, all header shorthand/conflict combinations,
and mixed historical record-version readers. Those additions should advance a
development CTS and later publish a new immutable snapshot; they do not mutate
snapshot 0.1.

Production-safe generic AET ingress, transaction-level integrity and encryption,
source-backed provenance mutation, index lifecycle closure, and head-aware ASP
storage remain separate unchecked requirements. Base portable event-stream
logical bytes now belong to `aes.integrity.v0`; that later completion does not
make them part of this earlier read/export and supported-application acceptance
run.
