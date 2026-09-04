# AES ecosystem impact checklist

Status: living working list

Updated: 2026-09-05

## Scope

This file tracks changes required across the AEON ecosystem as a result of the
portable AES event model currently being exercised by Telex. Telex is the
change driver, not the subject of this checklist.

In scope are shared contracts, language implementations, validators, query and
editing systems, operation and persistence layers, compatibility, CTS, and
rollout. Telex wire syntax, parsing, encoding profiles, and implementation
milestones are out of scope and remain in the Telex specification and
repository-local conformance notes.

Checkboxes describe implementation or specification work. Checked decision
statements are the current AES design baseline, not claims that every ecosystem
component already conforms.

## 0. Ecosystem release gates

Do not enable producers to write the revised event shape to durable or shared
surfaces until these gates are complete.

- [ ] Resolve the structural-identity contradiction between the AEON grammar
  and shared CTS. The grammar permits identity on attribute-entry and node
  heads, while current negative CTS cases require implementations to reject
  those locations.
- [ ] Publish one transport-neutral portable AES event contract covering path,
  kind, value, datatype, identity, attributes, ordering, and provenance.
- [ ] Define the bidirectional mapping between AEON source paths and portable
  AES event paths, including reference target payloads.
- [ ] Decide whether `aeon:header` participates in the portable event stream,
  is carried in a separate control-plane envelope, or is excluded.
- [ ] Define the source/origin identity required to make byte spans durable and
  portable.
- [x] Use encoding-neutral `aes.complete.v0` and `aes.partial.v0` profile
  discriminators so legacy and revised AES records cannot be confused; keep
  `telex.aes=0` as the independent wire-format version.
- [ ] Land shared CTS coverage for the agreed contract before implementations
  claim support.
- [ ] Define reader-first compatibility rules before any producer emits the
  revised shape into durable stores or cross-service interfaces.

## 1. Contract changes

### 1.1 Structural identity

#### Confirmed baseline

- [x] Carry an optional structural occurrence identity as `identity` on the
  flat event that represents the headed occurrence.
- [x] Omit AEON's surrounding backslashes from the AES payload.
- [x] Do not incorporate structural identity into canonical path or stream
  order.
- [x] Do not generate an identity merely to serialize an event.
- [x] Put an anonymous list, tuple, or node-child identity directly on that
  child's indexed event; do not emit a `typed-value` wrapper.
- [x] Put a node binding identity on the outer `node` event and a node-head
  identity on its indexed `node-head` event.

#### Specification and CTS

- [ ] Replace the shared CTS cases that reject identity on attribute-entry and
  node heads with positive preservation cases matching the AEON grammar.
- [ ] Define one portable identity grammar and one document-wide uniqueness
  rule across ordinary bindings, attribute entries, anonymous child heads, and
  node heads.
- [ ] Add duplicate-identity tests spanning all four head locations rather
  than testing each location in isolation.
- [ ] Add projection cases proving the identities at `$.a`, `$.a.@.x`,
  `$.a[0]`, and `$.a[0][0]` are preserved independently.

#### Implementation work

- [ ] TypeScript: add `structuralId` to `NodeLiteral` and consume identity
  after the node tag.
- [ ] TypeScript: add `structuralId` to `AttributeValue` and consume identity
  after the attribute-entry key.
- [ ] TypeScript: preserve attribute-entry identity through AES projection.
- [ ] TypeScript: audit canonical rendering, finalization, cloning, mode
  conversion, JSON projection, and mutation helpers for manufactured
  `structuralId: null` values or dropped identities.
- [ ] Rust: retain the existing ordinary-binding and anonymous-head support;
  add and verify attribute-entry and node-head identity support.
- [ ] Python: retain the existing ordinary-binding and anonymous-head support;
  add and verify attribute-entry and node-head identity support.
- [ ] PHP: inventory and implement all four identity locations in its AST,
  event projection, serialization, and validation surfaces.
- [ ] Verify SANSA projections, SO plans, ASP operations, and AES-DB records
  preserve identity without treating it as path identity.

### 1.2 Node projection and paths

#### Confirmed baseline

- [x] Represent a node as a value-less ordered `node` container.
- [x] Represent each node head as an indexed `node-head` event carrying its tag
  in `value`.
- [x] Current AEON produces one head at `[0]`; its content begins below that
  head at `[0][0]`, `[0][1]`, and so on.
- [x] Keep AES structurally capable of representing zero or multiple heads so
  future node forms do not require another event-model change.
- [x] Keep binding-head metadata on the outer `node` event and node-head
  metadata on the indexed `node-head` event.

#### Required ecosystem work

- [ ] Update TypeScript, Rust, Python, and PHP AEON-to-AES adapters to expand a
  `NodeLiteral` into an outer `node`, a `node-head`, and recursively flattened
  content events.
- [ ] Define AEON source-path to portable event-path translation for node
  descendants; the old first child path must never be silently reinterpreted
  as the new node-head path.
- [ ] Define translation and validation for clone-reference and
  pointer-reference target payloads across the additional node path level.
- [ ] Apply the same mapping to SANSA selections, Tonics edit addresses, SO/ASP
  mutation targets, diagnostics, and any public SDK navigation APIs.
- [ ] Give the node tag/head its own source span in ASTs that currently expose
  only the complete node-literal span, and define exactly which tokens the head
  span covers.
- [ ] Update portable kind registries and validators to add `node-head` and make
  `node` value-less.
- [ ] Update SANSA structural navigation and parent/container compatibility for
  `node[head-index][content-index]`.
- [ ] Update AEOS datatype and cardinality validation for the AEON requirement
  of exactly one node head.
- [ ] Update Tonics and other editing tools that address node-head metadata
  through node-specific commands or legacy child paths.

### 1.3 Flat attributes and descendants

- [x] Emit attribute values as ordinary flat events at canonical paths
  containing `.@`; do not embed attribute maps in the portable AES event.
- [x] Apply the same event shape recursively to nested attributes, object
  members, indexed values, node heads, and node content.
- [x] Leave attribute ownership and scope interpretation to SANSA and
  downstream consumers.
- [ ] Replace embedded attribute maps in TypeScript, Rust, Python, and PHP AES
  adapters and transport surfaces with flat projection or an explicit legacy
  compatibility adapter.
- [ ] Verify prefix-completeness and container-compatibility rules for
  attribute paths without synthesizing phantom parent bindings.
- [ ] Update SANSA, SO, ASP, AES-DB, validators, SDKs, and editing tools that
  currently expect attributes to be nested inside a parent event.

### 1.4 Datatypes and values

- [x] Carry the complete canonical datatype descriptor, including generic
  arguments and clarifiers, without the AEON `:`.
- [x] Keep a datatype only on the event where it was declared; do not infer or
  propagate container generic arguments onto descendants.
- [x] Use separate `clone-reference` and `pointer-reference` kinds with one
  canonical target-path payload shape.
- [x] Normalize trimtick content before AES and transport it as `kind=string`;
  delimiter width and indentation are source mechanics.
- [x] Add `wtc` as a distinct temporal kind.
- [x] Use lowercase `local` for WTC local temporal anchors.
- [x] Treat `conflictAuthority` as a consumer responsibility, not document or
  AES event authority.
- [x] Do not transport exact AEON lexemes or a generic representation field.
- [ ] Reconcile TypeScript, Rust, Python, PHP, ASP, AEOS, and CTS value-kind
  names and canonical payload rules with the portable table.
- [ ] Add WTC cases covering the three temporal anchor forms and local, named,
  and geographic references without introducing `conflictAuthority` into the
  portable event contract.
- [ ] Verify canonical payloads and semantic hashes preserve recognized value
  distinctions, including temporal distinctions, while excluding source-only
  spelling.
- [ ] Remove dependencies on implementation AST class names, raw tokens, and
  nested value trees at portable boundaries.

### 1.5 Spans and provenance

#### Confirmed direction

- [x] Use zero-based UTF-8 byte offsets with an inclusive start and exclusive
  end.
- [x] Transport `span=start:end`; do not transport derived line and column
  coordinates.
- [x] Omit spans for events without source evidence and never fabricate a zero
  span.

#### Required ecosystem work

- [ ] Define the portable source/origin identity contract. A durable byte span
  must refer to an immutable source revision, digest, or equivalent stable
  identifier.
- [ ] Define whether offsets include an accepted UTF-8 BOM and always measure
  against the exact, unnormalized source resource.
- [ ] Require span endpoints to fall on UTF-8 scalar boundaries.
- [ ] Derive line and column only when the identified source bytes are
  available.
- [ ] TypeScript: correct or replace the lexer claim that its UTF-16 string
  index is a byte offset, and convert positions at the portable boundary.
- [ ] Python: convert code-point offsets to UTF-8 byte offsets at the portable
  boundary.
- [ ] Rust: verify byte offsets and Unicode-scalar columns against shared
  non-ASCII fixtures.
- [ ] PHP: establish its current offset unit and convert it at the portable
  boundary where necessary.
- [ ] Update the AEON span appendix and CTS protocol, which currently use
  ambiguous or character-based offset language.
- [ ] Align portable provenance with the existing ASP `origin` shape, which
  carries `kind`, `source_id`, and `{start,end}`.
- [ ] Ensure SO uses spans only for diagnostics, audit, and provenance—not
  identity, ordering, mutation preconditions, or semantic decisions.

### 1.6 AEON headers and control-plane metadata

- [ ] Inventory how TypeScript, Rust, Python, and PHP currently expose
  `aeon:header` and shorthand `aeon:*` fields through AES.
- [ ] Decide whether headers are document events, a separate control-plane
  envelope, or excluded from portable AES.
- [ ] Define how the decision affects document completeness, ordering,
  round-trip behavior, semantic hashes, signatures, and profile selection.
- [ ] Add shared fixtures for structured headers, shorthand headers, header
  conflicts, and body-only streams.

## 2. Repository and component work

### Specifications and conformance

- [ ] `aeonite-specs`: update the canonical AES specification, which currently
  describes implementation-shaped AST values, required source spans, and
  embedded attributes.
- [ ] `aeonite-specs`: update AEON node, structural-identity, span, reference,
  datatype, and WTC projection requirements.
- [ ] `aeonite-cts`: replace contradictory identity vectors and add portable
  event-local, complete-stream, path, value, and provenance suites.
- [ ] `aeonite-cts`: require at least two independent implementations to pass
  each portable contract before promotion.

### Language implementations and public surfaces

- [ ] `altopelago/aeon`: update TypeScript, Rust, and Python parsers, ASTs,
  flatteners, materializers, SDKs, CLIs, and JSON/debug projections.
- [ ] `altopelago/aeon-php`: implement the same portable contract and shared
  CTS coverage rather than treating PHP as a later compatibility exercise.
- [ ] `altopelago/aeon-validator`: update validation and diagnostics for the
  revised event shape and path model.
- [ ] `altopelago/aeon-tooling`: inventory commands and interchange surfaces
  that consume or emit AES-shaped JSON.
- [ ] `altopelago/aeon-tonics`: update canonical rendering, formatting,
  conversion, editing addresses, and node/attribute handling.

### Semantic, operational, and persistence consumers

- [ ] SANSA: update structural navigation, ownership, scope, reference
  resolution, and source-path/event-path translation.
- [ ] AEOS: update datatype, cardinality, kind, node-head, and WTC validation.
- [ ] SO: add an explicit portable-AES adapter instead of treating current
  TypeScript AST-shaped values as the interchange contract.
- [ ] SO: update candidate construction, validation, stable-order scopes, and
  path-addressed operations for flat attributes and expanded nodes.
- [ ] ASP: preserve kind, canonical value, datatype, identity, and provenance
  without rebuilding source lexemes.
- [ ] ASP: update operations and origin handling for expanded paths and
  source-backed spans.
- [ ] AES-DB: reconstruct value-less containers and explicit descendants while
  remaining neutral on datatype inference, reference resolution, and
  value-family semantics.
- [ ] Audit relays, canonicalizers, signing paths, and durable codecs for
  assumptions about the legacy event shape.

## 3. Compatibility and stored-data migration

- [ ] Define a versioned compatibility projection from legacy
  `kind=node,value=tag` records to an outer `node` plus indexed `node-head`.
- [ ] Define compatibility behavior for stored events created before stable
  structural identity was required.
- [ ] Preserve legacy numeric node-child meaning; do not reinterpret it as a
  node-head path without an explicit versioned projection.
- [ ] Decide whether durable history remains in its original contract with a
  read-time compatibility view or is migrated into a new versioned store.
- [ ] Rebuild or invalidate AES-DB path, datatype, attribute, reference, and
  ordered-child indexes affected by the revised projection.
- [ ] Version persisted ASP/AES-DB records whose span units or checksummed
  source bytes change; do not silently rewrite historical logs.
- [ ] Verify subtree moves, deletion, replay, snapshots, checkpoints, backup,
  restore, and compaction preserve node-head and identity occurrences.
- [ ] Define the semantic-losslessness guarantee at the portable boundary.
  Exact source reconstruction requires retained source/provenance and must not
  be implied by a semantic event round trip alone.
- [ ] Define separate signature policies for canonical semantic projections
  and exact-order ledger streams.
- [ ] Ensure hashes and signatures bind the selected portable contract version
  and ordering policy so incompatible projections cannot be confused.

## 4. Cross-repository acceptance tests

- [ ] Structural identity: all four head locations, document-wide duplicates,
  and preservation through every supported language and consumer adapter.
- [ ] Nodes: nested, attributed, typed, empty, and multiple-head event streams;
  AEON-facing profiles must still enforce exactly one head.
- [ ] Paths and references: round trips across legacy and revised node paths,
  with no target silently changing meaning.
- [ ] Attributes: nested attribute descendants, duplicate paths,
  prefix-completeness, and container compatibility.
- [ ] Values: positive and negative cases for every kind and its
  allowed/required fields, including WTC anchor/reference variants.
- [ ] Spans: ASCII, non-ASCII, combining characters, astral characters, BOM,
  and absent-source fixtures across TypeScript, Rust, Python, and PHP.
- [ ] Headers: structured, shorthand, conflicting, and body-only inputs under
  the chosen control-plane policy.
- [ ] End to end: AEON source -> portable AES -> SO -> ASP -> AES-DB ->
  reconstructed portable AES.
- [ ] Compatibility: legacy readers reject or explicitly adapt revised records
  and revised readers accept supported legacy records without ambiguity.

## 5. Rollout sequence

- [ ] Phase 1 — freeze and publish the transport-neutral event contract and
  its version discriminator.
- [ ] Phase 2 — update `aeonite-specs` and land shared CTS vectors.
- [ ] Phase 3 — ship compatibility readers/adapters in TypeScript, Rust,
  Python, and PHP while producers retain the legacy shape.
- [ ] Phase 4 — update SANSA, AEOS, Tonics, validators, SDKs, SO, ASP, AES-DB,
  relays, canonicalizers, and signing/hashing consumers.
- [ ] Phase 5 — provide versioned read views or migrations for persisted data,
  then rebuild or invalidate affected indexes.
- [ ] Phase 6 — run the cross-repository acceptance suite and verify mixed
  legacy/revised deployments.
- [ ] Phase 7 — enable revised producers only after all required readers and
  durable consumers have passed compatibility checks.
- [ ] Review every completed ecosystem decision for additional repository,
  migration, and CTS work before closing the rollout.

## Deferred: Telex-specific work

The following belongs to the Telex specification and repository-local work,
not this ecosystem sweep:

- Wire grammar, separators, escaping, BOM handling, and exact serialization.
- Telex-specific stream headers, extension-field negotiation, and codec
  versioning.
- Telex partial/complete validation modes and their names.
- JavaScript/Rust Telex codec parity and Draft 0 implementation milestones.
- Telex fixture promotion except where a fixture becomes a transport-neutral
  AES or shared CTS case.
