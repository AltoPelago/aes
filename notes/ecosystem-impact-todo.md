# AES ecosystem impact checklist

Status: living working list

Updated: 2026-09-04

This file records AES decisions and implementation findings that require work
outside this repository. It is intentionally broader than the Telex Draft 0
decision-gate list. Completing a Telex syntax codec does not complete these
items.

Checkboxes describe implementation or specification work. Checked decision
statements are the current AES design baseline, not claims that every ecosystem
component already conforms.

## 1. Structural identity

### Confirmed model

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

### TypeScript deficiencies found

- [ ] Add `structuralId` to `NodeLiteral`. The AEON grammar permits structural
  identity after a node tag, but the TypeScript AST currently has no field for
  it and `parseNode()` does not consume it.
- [ ] Add `structuralId` to `AttributeValue`. The grammar permits structural
  identity after an attribute-entry key, but the TypeScript attribute parser
  currently skips that position.
- [ ] Preserve attribute-entry identity through the TypeScript AES projection;
  its current `AttributeEntry` surface carries value, datatype, and nested
  attributes but no structural identity.
- [ ] Audit canonical rendering, finalization, cloning, mode conversion, JSON
  projection, and mutation helpers for places that manufacture
  `structuralId: null` or otherwise drop an existing identity.
- [ ] Add parser and round-trip tests for identities on ordinary bindings,
  attribute entries, anonymous child heads, and node heads.
- [ ] Add document-wide duplicate-identity tests spanning all four supported
  head locations, rather than checking each location in isolation.
- [ ] Add AES flattening tests proving the identity locations `$.a`,
  `$.a.@.x`, `$.a[0]`, and `$.a[0][0]` are preserved independently.

### Cross-implementation work

- [ ] Audit the Rust and Python AST/event surfaces for all structural-identity
  locations. The initial inventory found no event-level parity with the recent
  TypeScript field.
- [ ] Define one portable identity grammar and uniqueness rule in the AES/AEON
  specifications and add shared CTS vectors.
- [ ] Verify that SANSA projections, SO plans, ASP operations, and AES-DB
  records preserve identity without treating it as path identity.
- [ ] Define compatibility behavior for stored events created before a profile
  required stable structural identity.

## 2. Node projection

### Confirmed model

- [x] Represent a node as a value-less ordered `node` container.
- [x] Represent each node head as an indexed `node-head` event carrying its tag
  in `value`.
- [x] Current AEON produces one head at `[0]`; its content begins below that
  head at `[0][0]`, `[0][1]`, and so on.
- [x] Keep AES structurally capable of representing zero or multiple heads so
  future forms equivalent to `<>` and `<<tag>, <tag>>` do not require a new
  event model.
- [x] Keep binding-head metadata on the outer `node` event and node-head
  metadata on the indexed `node-head` event.

### Required ecosystem changes

- [ ] Update AEON-to-AES adapters to expand the current single `NodeLiteral`
  event into a `node` event, a `node-head` event, and recursively flattened
  content events.
- [ ] Give the node tag/head its own narrow source span in ASTs that currently
  expose only the span of the complete node literal.
- [ ] Update portable kind registries and validators to add `node-head` and make
  `node` value-less.
- [ ] Update SANSA structural navigation and parent/container compatibility for
  `node[head-index][content-index]`.
- [ ] Update reference validation and materialization for the additional node
  path level.
- [ ] Update AEOS datatype and cardinality validation: current AEON requires
  exactly one head, while Telex syntax itself does not impose that profile.
- [ ] Update Tonics and editing tools that currently address node-head metadata
  through node-specific commands or the old child paths.
- [ ] Add nested-node, attributed-head, typed-head, empty-node, and multi-head
  conformance vectors.

### Stored-data compatibility

- [ ] Define a versioned compatibility projection from legacy
  `kind=node,value=tag` records to outer `node` plus indexed `node-head`
  records.
- [ ] Do not silently reinterpret old numeric node-child paths as new
  node-head paths.
- [ ] Rebuild or invalidate AES-DB path, datatype, attribute, reference, and
  ordered-child indexes affected by node path expansion.
- [ ] Verify subtree moves, deletion, replay, snapshots, checkpoints, backup,
  restore, and compaction preserve the new node-head occurrence.
- [ ] Decide whether durable history remains in its original profile with a
  read-time compatibility view or is migrated into a new versioned store.

## 3. Flat attributes and descendants

- [x] Emit attribute values as ordinary flat events at canonical paths
  containing `.@`; do not embed attribute maps in the portable AES event.
- [x] Apply the same event shape recursively to nested attributes, object
  members, indexed values items, node heads, and node content.
- [x] Leave attribute ownership and scope interpretation to SANSA and
  downstream consumers.
- [ ] Replace embedded attribute maps in existing AES adapters and transport
  surfaces with flat event projection or an explicit compatibility adapter.
- [ ] Verify prefix-completeness and container-compatibility rules for attribute
  paths without synthesizing phantom parent bindings.
- [ ] Add duplicate-path and nested-attribute CTS cases across all supported
  parent kinds.

## 4. Datatypes and values

- [x] Carry the complete canonical datatype descriptor, including generic
  arguments and clarifiers, without the AEON `:`.
- [x] Keep a datatype only on the event where it was declared; do not infer or
  propagate container generic arguments onto descendants.
- [x] Use separate `clone-reference` and `pointer-reference` kinds with one
  canonical target-path payload shape.
- [x] Normalize trimtick content before AES and transport it as `kind=string`;
  delimiter width and indentation are source mechanics.
- [x] Add `wtc` as a distinct temporal kind.
- [x] Do not transport exact AEON lexemes or a generic representation field.
- [ ] Reconcile TypeScript, Rust, Python, ASP, AEOS, and CTS value-kind names and
  canonical payload rules with the portable table.
- [ ] Remove dependencies on implementation AST class names, raw tokens, and
  nested value trees at portable boundaries.
- [ ] Add positive and negative conformance vectors for every kind and its
  allowed/required fields.
- [ ] Verify semantic hashes exclude source-only spelling while preserving all
  recognized AES value distinctions.

## 5. Spans and provenance

### Confirmed direction

- [x] Use zero-based UTF-8 byte offsets with an inclusive start and exclusive
  end.
- [x] Transport the compact form `span=start:end`; do not transport derived
  line and column coordinates.
- [x] Omit spans for events without source evidence and never fabricate a zero
  span.

### Findings and work

- [ ] Correct or replace the TypeScript lexer claim that its current offset is
  a byte offset. JavaScript string indexing currently makes it a UTF-16 code
  unit offset.
- [ ] Convert TypeScript UTF-16 positions to UTF-8 byte offsets at the portable
  AES boundary, or change the lexer while preserving compatibility for existing
  consumers.
- [ ] Convert Python code-point offsets to UTF-8 byte offsets at the portable
  AES boundary.
- [ ] Verify Rust byte offsets and Unicode-scalar columns against shared
  non-ASCII fixtures.
- [ ] Update the AEON span appendix and CTS protocol, which currently use
  ambiguous or character-based offset language.
- [ ] Define whether offsets include an accepted UTF-8 BOM and always measure
  against the exact, unnormalized source resource.
- [ ] Require span endpoints to fall on UTF-8 scalar boundaries.
- [ ] Derive line and column only when the identified source bytes are
  available.
- [ ] Decide the portable source/origin identity contract. A durable byte span
  must refer to an immutable source revision, digest, or equivalent stable
  source identifier.
- [ ] Align Telex provenance with the existing ASP `origin` shape, which already
  carries `kind`, `source_id`, and `{start,end}`.
- [ ] Version persisted ASP/AES-DB records whose span units or checksummed bytes
  change; do not rewrite historical logs silently.
- [ ] Ensure SO uses spans only for diagnostics, audit, and provenance—not
  identity, ordering, mutation preconditions, or semantic decisions.

## 6. SO, ASP, and AES-DB integration

- [ ] Add an explicit portable-AES adapter at the SO/ASP boundary instead of
  treating current TypeScript AST-shaped values as the interchange contract.
- [ ] Update SO candidate construction and validation for flat attributes and
  expanded node paths.
- [ ] Update ASP codecs and operations to preserve `kind`, canonical `value`,
  `datatype`, `identity`, and provenance without rebuilding source lexemes.
- [ ] Update AES-DB reconstruction and materialization for value-less
  containers and explicit descendant events.
- [ ] Verify stable-order planning treats the node-head container and each
  node-head content sequence as distinct ordering scopes.
- [ ] Ensure generated SO/ASP/AES-DB events omit `span` unless they retain
  genuine source evidence.
- [ ] Keep AES-DB semantic-policy neutral: it stores and reconstructs the AES
  contract but does not infer datatypes, resolve references, or interpret
  value-family semantics.
- [ ] Add an end-to-end fixture covering AEON source -> portable AES -> SO ->
  ASP -> AES-DB -> reconstructed portable AES.

## 7. Specifications, CTS, and rollout

- [ ] Add an explicit AES projection requirement to `aeon.gp.profile.v1` by
  referencing `aes.telex.v0`; do not duplicate its normative structural rules
  in the AEON GP contract. Telex selects the same complete profile when its
  stream header omits `profile`.
- [ ] Audit relays, canonicalizers, durable codecs, and signing paths to ensure
  unknown Telex fields are preserved or rejected, never silently discarded.
- [ ] Register exact extension fields in negotiated profiles and reject all
  unregistered `x.*` fields at semantic boundaries.
- [ ] Define separate signature profiles for canonical semantic projections
  and exact-order ledger streams; do not make Telex canonicalization reorder
  events implicitly.
- [ ] Ensure hashing and signature implementations bind the selected AES
  profile and order policy so signatures from different projections cannot be
  confused.
- [ ] Update the canonical AES specification, which currently describes an
  implementation-shaped `ASTValue`, required source span, and embedded
  attributes.
- [ ] Update AEON node and structural-identity specifications to match the
  implemented grammar and the portable AES projection.
- [ ] Publish `aes.telex.v0` (complete and the default) and `aes.raw.v0`
  (explicitly unconstrained) so legacy and portable AES records cannot be
  confused.
- [ ] Define reader-first rollout and compatibility rules before any producer
  emits the new node projection into durable stores.
- [ ] Add shared Telex fixtures and require at least two independent
  implementations before Draft 1.
- [ ] Review every completed decision gate for additions to this checklist.
