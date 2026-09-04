# AES

AES is the shared Assignment Event Stream layer of the Aeonite ecosystem.

This repository gives AES an independent home. It will contain the portable
event model, interchange formats, conformance material, and the Aeonic Semantic
Language used by AES-family technologies.

The first deliverable is [`telex.aes`](specifications/telex.aes.md), a small
text format for exchanging AES events between implementations. `film.aes` will
later provide a binary encoding of the same event model. Neither format defines
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

This is a spec-first bootstrap. Nothing in this repository is normative yet.

- [Telex Draft 0](specifications/telex.aes.md) proposes the textual framing and
  the first portable event profile.
- [Current event-model inventory](notes/current-event-model.md) records the
  differences that exist across the current TypeScript, Rust, and Python AEON
  implementations.
- [AES ecosystem impact checklist](notes/ecosystem-impact-todo.md) tracks
  specification, implementation, conformance, and migration work caused by the
  portable event-model decisions.
- [`src/telex.js`](src/telex.js) is a dependency-free syntax codec. It proves
  that the framing can be parsed and produced with a very small implementation;
  it deliberately does not decide AES value semantics. It preserves an
  explicit stream profile, and its separate `checkTelexCompleteness` helper
  reports missing structural prefixes without claiming full `aes.complete.v0`
  validation. `validateTelex` and `validateTelexRecords` apply the event-local
  Draft 0 rules and the selected profile's structural checks. AEON headers are
  absent by default; `projection=aeon.document.v0` explicitly enables flat
  `header=` control records in a separate address plane. Optional provenance
  uses a record-local `origin=sha256:<digest>` and permits `span` only alongside
  that origin.
- [`examples/customer.telex.aes`](examples/customer.telex.aes) is an early
  illustrative stream, not a frozen conformance vector.
- [`conformance/`](conformance/README.md) contains language-neutral Draft 0
  vectors for syntax, canonicalization, AES profile validation, and the optional
  AEON document projection.
- [`implementations/rust/`](implementations/rust/README.md) is an independent,
  dependency-free Rust implementation of the same Draft 0 contract.

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

## Boundaries

AES is the event model. Telex and Film are encodings of that model.

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

1. Reconcile the portable AES event model with existing implementations.
2. Fix Telex framing, field vocabulary, escaping, and canonical bytes.
3. Publish shared positive and negative conformance vectors.
4. Prove round trips in at least two independent implementations.
5. Promote `telex.aes` from draft and use its event model to design `film.aes`.
6. Consolidate the AES, Telex, Film, and later Tape specifications here.
7. Consolidate shared value behavior as the Aeonic Semantic Language.

## Design rule

One semantic contract should have one owner. Implementations may keep ergonomic
or derived fields internally, but those fields do not become portable merely
because they appear in a JSON debug response.
