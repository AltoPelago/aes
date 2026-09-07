# AES

AES is the shared Assignment Event Stream layer of the Aeonite ecosystem.

This repository gives AES an independent home. It will contain the portable
event model, interchange formats, conformance material, and the Aeonic Semantic
Language used by AES-family technologies.

The [portable AES event contract](specifications/aes.events.md) defines the
shared model. [`telex.aes`](specifications/telex.aes.md) is its first encoding:
a small text format for exchanging records between implementations. `film.aes`
will later provide a binary encoding of the same model. Neither format defines
different event semantics.

AES is semantically lossless relative to a selected event profile and
projection, not relative to original source bytes. Telex preserves portable
records and provenance, while exact AEON spelling and layout require the
separately retained source artifact.

```text
AEON and other producers
          |
          v
  portable AES events
       /       \
      v         v
telex.aes    film.aes
  text         binary
```

## Current status

This repository remains the implementation-adjacent AES workspace. Canonical
publication sources live under `sources/aes/v0/` in
[`aeonite-specs`](../../aeonite-org/aeonite-specs/). The AES v0 index, portable
event contract, compatibility contract, and Telex encoding are published;
their declared lifecycle and normativity govern v0. The Aeonic Semantic
Language remains a proposal. The integrity and Assignment Event Transaction
contracts are normative drafts. The Markdown documents here are
working/reference copies and do not override those canonical sources.

- [Portable AES Event Contract v0](specifications/aes.events.md) owns the
  transport-neutral record, profile, projection, and fidelity rules.
- [Portable AES Compatibility Contract v0](specifications/aes.compatibility.md)
  defines explicit legacy adapters, durable read views, and the
  reader-before-writer deployment barrier.
- [Portable AES Integrity Contract v0](specifications/aes.integrity.md) defines
  encoding-neutral logical bytes, SHA-256 digests, canonical-semantic and exact
  ordering policies, optional provenance coverage, and domain-separated
  signature inputs without treating Telex or Film bytes as the signed form.
- [Assignment Event Transaction Contract v0](specifications/aes.transactions.md)
  defines the non-actionable logical transaction envelope, exact-order
  transaction integrity, trusted-host boundaries, source-backed preparation,
  and the initial narrow ASP scalar-value replacement application without
  changing Telex framing.
- [Telex v0](specifications/telex.aes.md) owns textual framing, escaping,
  canonical bytes, and syntax diagnostics.
- [Film v0 binary encoding proposal](proposals/film.aes.md) records the
  transport boundary, initial streaming direction, performance contract, and
  deferred wire-format decision gates for the binary AES encoding.
- [Current event-model inventory](notes/current-event-model.md) records the
  differences that exist across the current TypeScript, Rust, and Python AEON
  implementations.
- [AES ecosystem impact checklist](notes/ecosystem-impact-todo.md) tracks
  specification, implementation, conformance, and migration work caused by the
  portable event-model decisions.
- [AltoPelago Aeonic Limits v1](notes/altopelago-aeonic-limits-v1.md) defines
  the informative, consumer-owned configuration shape used to align structural
  and processing limits across AltoPelago implementations. Format byte limits
  remain local to AEON, Telex, Film, or their enclosing transport, and schema
  budgets remain separate. The concrete `1.0.0` set is
  [`policies/altopelago.aeonic-limits.v1.aeon`](policies/altopelago.aeonic-limits.v1.aeon).
- [`src/telex.js`](src/telex.js) is a dependency-free syntax codec. It proves
  that the framing can be parsed and produced with a very small implementation;
  it deliberately does not decide AES value semantics. It preserves an
  explicit stream profile, and its separate `checkTelexCompleteness` helper
  reports missing structural prefixes without claiming full `aes.complete.v0`
  validation. `validateTelex` and `validateTelexRecords` apply the event-local
  v0 rules and the selected profile's structural checks. AEON headers are
  absent by default; `projection=aeon.document.v0` explicitly enables flat
  `header=` control records in a separate address plane. Optional provenance
  uses a record-local `origin=sha256:<digest>` and permits `span` only alongside
  that origin. Its decoder expands compact Telex datatype lines into logical
  `datatype`, `generics`, and `clarifiers` components. Recursive generics are
  guarded by consumer-selected processing limits and are never truncated. The
  codec exports `DEFAULT_TELEX_LIMITS` and accepts normalized limits through
  its public parse, encode, canonicalize, and validation options; resolving an
  AEON limits file remains the trusted caller's responsibility.
- [`src/provenance.js`](src/provenance.js) audits record-local origin digests
  and UTF-8 byte spans against exact caller-retained bytes. It reports artifact
  availability separately from evidence validity and gates
  `aes.preparation.source-backed.v0` without turning source coordinates into
  mutation authority.
- [`examples/customer.telex.aes`](examples/customer.telex.aes) is an early
  illustrative stream, not a frozen conformance vector.
- [`conformance/`](conformance/README.md) contains language-neutral v0
  vectors for syntax, canonicalization, AES profile validation, exact-source
  provenance, resource-limit boundaries, and the optional AEON document
  projection. Stable external
  targets are published separately as `aes-events-cts-v0-snapshot-0.1` and
  `telex-cts-v0-snapshot-0.1` in the shared `aeonite-cts` repository.
- [`implementations/rust/`](implementations/rust/README.md) is an independent
  Rust implementation of the same v0 event, Telex, integrity, provenance, and
  transaction contracts.
  Its Telex codec has no runtime dependencies; portable SHA-256 integrity uses
  the audited `sha2` crate.

Run the syntax tests with:

```bash
npm test
```

Run only the portable vector harness with:

```bash
npm run test:conformance
```

Run the vectors through the Rust implementation with:

```bash
npm run test:conformance:rust
```

Run both published shared snapshots in JavaScript and Rust with:

```bash
npm run test:conformance:shared
npm run test:conformance:rust:shared
```

## Boundaries

AES is the event model. Telex and Film are encodings of that model.

The portable event contract is authoritative for fields, value kinds, flat
structure, paths, identity, provenance, ordering, completeness, and projections.
Encoding specifications reference that contract instead of redefining it.

`telex.aes` is not AEON source syntax, JSON with a new extension, a materialized
object tree, or a schema language. A decoder should be able to reconstruct a
portable AES event without running AEON Core or inferring a value type.

AES is flat. Every represented value—including object members, list and node
items, and values reached through an attribute address space—has its own event
at a canonical path. AES does not embed an attribute tree or assign special
meaning to `.@`; address-space interpretation belongs to SANSA and downstream
consumers.

The Aeonic Semantic Language will centralize the shared meaning of AES values:
equality, comparison, ordering, conversion, measurement, and later arithmetic.
It is downstream of value recognition and shared by AEOS, SANSA, Tonics,
storage, and other consumers. Its specification is a later repository stage;
Telex only needs the value-kind vocabulary required to carry values without
ambiguity.

## Roadmap

1. Continue compatibility readers and adapters across the existing producers
   and consumers.
2. Prototype and resolve the decision gates in the
   [Film v0 binary encoding proposal](proposals/film.aes.md).
3. Consolidate the later Film and Tape specifications here.
4. Advance shared value behavior through the Aeonic Semantic Language proposal.

## Design rule

One semantic contract should have one owner. Implementations may keep ergonomic
or derived fields internally, but those fields do not become portable merely
because they appear in a JSON debug response.
