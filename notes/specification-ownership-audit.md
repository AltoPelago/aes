# AES specification ownership audit

Status: working publication audit

Updated: 2026-09-05

## Purpose

This audit identifies specification material that should move into the new AES
family, material that should remain with its present language or consumer, and
published text that must be reconciled with the portable `aes.events.v1`
contract.

The classification rule is responsibility, not which syntax first exposed a
concept:

- AEON owns source grammar, source legality, lexical recognition, and the
  mapping from AEON source constructs into portable events.
- AES owns the transport-neutral event model, portable representation-kind
  vocabulary, profiles, projections, ordering and fidelity contracts, and its
  encodings.
- the Aeonic Semantic Language owns shared operations over represented values
  after a valid representation exists.
- AEOS, SANSA, Tonics, ASP, AES-DB, and other consumers own how those shared
  contracts are applied in their domains.

## First-class AES publication set

The publication set now lives under `aeonite-specs/sources/aes/v1/`:

| Document | Lifecycle | Responsibility |
| --- | --- | --- |
| AES v1 index and architecture | draft | family boundaries, document map, versioning, and conformance links |
| Portable AES Event Contract | draft, normative | `aes.events.v1`, records, paths, kinds, profiles, projections, ordering, provenance, fidelity, and diagnostics |
| Telex AES | draft, normative | `telex.aes=1` textual framing, escaping, canonical bytes, and syntax diagnostics |
| Portable AES Compatibility Contract | draft, normative | legacy adapters, explicit source contracts, reader-first rollout, and durable-data rules |
| Aeonic Semantic Language | proposal, mixed | equality, comparison, ordering, conversion, measurement, and later arithmetic over represented values |

Film and Tape should appear in the index as planned surfaces without empty
normative specifications.

## Material that should move

### Shared AEON Value Semantics

`sources/aeon/v1/value-semantics-v1.aeon` should become the initial Aeonic
Semantic Language proposal in the AES family.

The document already places its work after Core has recognized a value and
defines behavior shared by AEOS, SANSA, Tonics, storage, and future consumers.
That is not AEON source-language ownership. Its equality, comparison,
collation, case mapping, conversion, measurement, arithmetic, structural
equality, reference-form comparison, temporal comparison, and consumer-handoff
sections belong together under the Aeonic Semantic Language.

Moving the ownership does not move these responsibilities:

- AEON retains literal grammar, datatype syntax, source canonicalization, and
  source-to-event projection.
- SANSA retains resolution, selector equivalence, Binding Sets, query flow, and
  mutation planning.
- AEOS retains schema rules and validation diagnostics.
- hosts and trusted profiles retain authorization and authority-bearing policy.

The current proposal has already been named by a released CTS scaffold. Do not
rewrite that historical target in place. Publish the AES-family successor with
a new document identity, retain the old proposal as a superseded historical
record with a prominent successor notice, and point current relationships to
the successor. Existing
`aeon.value.*` profile identifiers may remain compatibility names; changing
their spelling is a separate semantic-versioning decision, not a consequence
of moving the document.

### Assignment Event Stream appendix

`sources/appendices/v1/appendix-aes.aeon` currently acts as the AES contract.
Its normative ownership should move to the first-class Portable AES Event
Contract. The appendix should become a superseded historical record with a
prominent compatibility notice rather than a second current AES definition.

This avoids silently rewriting a published document while removing competing
authority. The new contract must supersede these outdated claims:

- an event contains an embedded `ASTValue`;
- `span` is required without an origin identity;
- attributes are embedded on their owning event;
- indexed children are merely optional synthetic events;
- every stream preserves lexical source order as its only ordering policy;
- AES is unqualifiedly lossless relative to original source;
- one event is emitted only for each source binding.

## Material that should stay and reference AES

| Current owner | Material that stays | Required reconciliation |
| --- | --- | --- |
| AEON Core and value types | literal syntax, aliases, datatype syntax, canonical source payloads, and recognition | map recognized values to normative AES representation kinds; refer semantic operations to the Aeonic Semantic Language |
| AEON structure syntax | object/list/tuple/node grammar, attributes, structural identity syntax | define AEON-source to AES-event projection; do not redefine the portable record shape |
| AEON addressing and references | AEON exact-path grammar and source reference legality | retain the bidirectional AEON/AES path translation, including node-head insertion and reference payload translation |
| AEON headers and conventions | structured/shorthand header source syntax and document policy | refer to body-only default projection and explicit `aeon.document.v1` header plane |
| AEON processing model | AEON compilation phases and fail-closed production | remove its duplicate event interface and examples; refer to `aes.events.v1` for emitted shape |
| AEON node appendix | node source syntax, child grammar, and AEON-facing single-head rule | replace optional synthetic children with the defined `NodeLiteral`/`NodeHead` projection and current path mapping |
| AEON spans appendix | source locations and diagnostic targeting | keep producer-side span meaning; refer portable carriage, origin binding, byte units, and audit rules to AES |
| Integrity envelope | envelope syntax, trust and verification policy | select an explicit AES projection/order/signature scope; correct the claim that AEON headers are body state |
| AEOS | schema vocabulary, validation behavior, and diagnostics | consume a named AES contract/profile; remove the undeclared `AES.meta.recovery_mode` and generic “read-only ledger” model |
| SANSA | addressing, resolving, querying, and mutation planning | consume AES-backed namespaces and the Aeonic Semantic Language; identities remain metadata rather than path identity |
| Tonics | runtime materialization | consume portable kinds and shared semantics without redefining either |
| Contracts and conventions | domain/profile declarations | bind explicitly to applicable AES and semantic-language versions where behavior depends on them |

## Published text requiring correction

### High priority: contradicts the portable event model

1. `appendix-aes.aeon` defines the legacy nested AST-shaped event contract.
2. `appendix-processing-model.aeon` requires `key`, embedded `value`, required
   `span`, optional `annotations` and `raw`, and says child events are optional.
3. `appendix-node-model.aeon` omits the node-head event and currently maps the
   former first child directly to `$.p[0]`; portable AES maps the node head to
   `$.p[0]` and the former first child to `$.p[0][0]`.
4. `appendix-aeos-charter.aeon` assumes an undeclared stream metadata object,
   permits duplicate paths through recovery metadata, and treats every AES as
   a ledger. Portable AES instead uses explicit complete/partial profiles and
   leaves ledger policy to an enclosing protocol or consumer.
5. `appendix-integrity-envelope.aeon` says convention headers are body state.
   Portable AES excludes headers from the default body projection and includes
   them only through `aeon.document.v1` and an explicitly broader signature
   scope.

### Medium priority: boundary or terminology drift

1. SANSA draft/proposal overview documents say “AES defines how meaning is
   persisted.” AES defines a portable event representation; persistence and
   history belong to ASP, AES-DB, a ledger protocol, or another host.
2. `aeon-gp-assertion-v1.aeon` says AES may record replacement, movement,
   removal, provenance, and history. Baseline AES represents ordered events but
   does not itself define mutation/history operations; that sentence needs an
   explicit ledger or persistence qualifier.
3. Several documents use “lossless” without stating whether they mean event
   fidelity, profile-relative semantic fidelity, or exact source round-trip
   fidelity.
4. Several documents use “attributes” as attached maps. Portable AES represents
   attribute values as ordinary flat events in the `.@` address space.
5. Several documents describe AES kinds with lowercase consumer labels or
   implementation AST objects. Portable boundaries must use the normative
   PascalCase representation-kind vocabulary.

### Lower priority: reference updates

References to `appendix-aes-v1` and `aeon-v1-value-semantics` occur throughout
AEON, AEOS, SANSA, appendices, publication example metadata, and CTS coverage.
They should be redirected to the new AES documents where the reference is to
current behavior. Historical snapshot references must remain intact.

## Concepts that should not move merely because AES carries them

- Structural identity syntax remains AEON-owned. AES preserves its payload and
  document-wide uniqueness under complete profiles.
- Canonical AEON path syntax remains AEON/SANSA-owned. AES owns which portable
  event addresses exist and the event-profile structural rules.
- Datatype meaning remains profile/schema/semantic-language-owned. AES carries
  the base name, recursive generics, and tagged clarifiers without inference.
- Header meaning remains convention/consumer-owned. AES only defines the
  optional control-plane projection.
- Source spans originate with source producers. AES defines portable span and
  origin carriage, not how a parser locates a token.
- Reference resolution and pointer mutation authority remain downstream. AES
  preserves reference kind and target path and checks target presence only
  where the selected completeness profile claims it.

## Publication and migration sequence

1. Add the new AES-family documents as draft/proposal sources.
2. Add the AES website family page and navigation generated from those sources.
3. Convert the old AES appendix and AEON value-semantics proposal into
   superseded records pointing to their new owners.
4. Reconcile the high-priority contradictions, then update ordinary references
   and terminology.
5. Keep the local Telex CTS mutable while v1 evolves.
6. At release, copy the exact reviewed suite and specifications into the shared
   CTS/spec snapshot, record content digests, mint new immutable snapshot IDs,
   and require two independent passing implementations.

Draft website publication can begin before step 6. Claims of released portable
AES conformance cannot.
