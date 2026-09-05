# AES ecosystem impact checklist

Status: living working list

Updated: 2026-09-05

## Scope

This file tracks changes required across the AEON ecosystem as a result of the
portable AES event model currently being exercised by Telex. Telex is the
change driver, not the subject of this checklist.

In scope are shared contracts, language implementations, validators, query and
editing systems, operation and persistence layers, compatibility, CTS, and
rollout. Telex wire syntax, parsing, carriage of AES context, and implementation
milestones are out of scope and remain in the Telex specification and
repository-local conformance notes.

Checkboxes describe implementation or specification work. Checked decision
statements are the current AES design baseline, not claims that every ecosystem
component already conforms.

## 0. Ecosystem release gates

Do not enable producers to write the revised event shape to durable or shared
surfaces until these gates are complete.

- [x] Resolve the structural-identity contradiction between the AEON grammar
  and shared CTS. The grammar now permits identity on attribute-entry and node
  heads, and the former negative CTS cases have been replaced.
- [x] Publish one transport-neutral portable AES event contract covering path,
  kind, value, datatype components, identity, attributes, ordering, and provenance in
  `specifications/aes.events.md`.
- [x] Define the bidirectional mapping between AEON source paths and portable
  AES event paths, including reference target payloads.
- [x] Exclude AEON headers from the default body event stream and carry them
  only through the explicit, encoding-neutral `aeon.document.v0` projection,
  using flat `header` records in a disjoint control-plane address space.
- [x] Make `origin` and `span` optional record-local provenance. Draft 0 uses
  `origin=sha256:<64 lowercase hex>` over exact source bytes, and rejects a
  `span` that has no `origin`.
- [x] Identify the portable event model as `aes.events.v0`; use the independent,
  encoding-neutral `aes.complete.v0` and `aes.partial.v0` completeness profiles;
  and keep `telex.aes=0` as the wire-format version that maps to the contract.
- [ ] Land shared CTS coverage for the agreed contract before implementations
  claim support.
- [x] Define reader-first compatibility rules before any producer emits the
  revised shape into durable stores or cross-service interfaces.

### Latest contract consistency audit

The 2026-09-05 audit extracted transport-neutral semantics into
`specifications/aes.events.md` and reduced the Telex document to encoding
concerns. Conformance metadata now points each suite at its owning specification.
Repository tests verify that every referenced specification heading resolves
and that the documented core fields, value kinds, and semantic diagnostic codes
remain aligned with the JavaScript reference validator.

The remaining unchecked release gate is shared CTS promotion. Reader-first
conversion, persistence, capability, and writer-activation rules are now
defined in `specifications/aes.compatibility.md`; their ecosystem implementation
and acceptance work remains open below.

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
- [x] Put a node binding identity on the outer `NodeLiteral` event and a node
  head identity on its indexed `NodeHead` event.

#### Specification and CTS

- [x] Replace the shared CTS cases that reject identity on attribute-entry and
  node heads with positive preservation cases matching the AEON grammar.
- [x] Define one portable identity grammar and one document-wide uniqueness
  rule across ordinary bindings, attribute entries, anonymous child heads, and
  node heads.
- [x] Add duplicate-identity tests spanning all four head locations rather
  than testing each location in isolation.
- [x] Add projection cases proving the identities at `$.a`, `$.a.@.x`,
  `$.a[0]`, and `$.a[0][0]` are preserved independently. These are staged in
  the experimental `aes-cts-v1-snapshot-0.3` next manifest; activation remains
  gated on the flat-attribute and expanded-node adapter work below.

#### Implementation work

- [x] TypeScript: add `structuralId` to `NodeLiteral` and consume identity
  after the node tag.
- [x] TypeScript: add `structuralId` to `AttributeValue` and consume identity
  after the attribute-entry key.
- [x] TypeScript: preserve attribute-entry identity through AES projection.
- [x] TypeScript: audit canonical rendering, finalization, cloning, mode
  conversion, JSON projection, and mutation helpers for manufactured
  `structuralId: null` values or dropped identities.
- [x] Rust: retain the existing ordinary-binding and anonymous-head support;
  add and verify attribute-entry and node-head identity support.
- [x] Python: retain the existing ordinary-binding and anonymous-head support;
  add and verify attribute-entry and node-head identity support.
- [x] PHP: inventory and implement all four identity locations in its AST,
  event projection, serialization, and validation surfaces.
- [x] Verify SANSA projections preserve occurrence identity independently of
  canonical address. JavaScript and Python resolution retain the original host binding,
  Rust carries `identity` on resolved bindings, and AEON Matter exposes all four
  source identity locations without changing selectors or canonical addresses.
- [x] Verify SO plans preserve identity independently of source and target
  addresses.
  Compatibility values now carry portable `identity` metadata independently
  of source and target paths. Exact-alias relocation preserves a single
  identity and fails closed on conflicting identities; derived identity policy
  remains explicit in the transform/projector.
- [x] Verify ASP operations preserve identity independently of operation target
  addresses.
  Identity-bearing assignment, attribute-operation, and nested attribute-entry
  writes now validate and retain `structuralId` through ASP storage, compact
  transport, candidate reconstruction, AEOS adaptation, and SANSA replacement
  lowering without deriving identity from the target path. Portable mapping for
  expanded node heads and anonymous children remains tracked separately below.
- [x] Verify AES-DB records preserve identity independently of persisted record
  paths and physical storage layout.
  Binding and nested attribute identities now have end-to-end coverage across
  canonical relocation, compact AEON logs, verified snapshots, recovery and
  retained checkpoints, suffix replay, compaction, backup, point-in-time
  restore, and restore into a different filesystem root. The compact profile
  decoder was corrected so omitted optional datatype metadata no longer blocks
  replay of identity-bearing records.

The TypeScript local AST/AES audit is complete. Canonical rendering, minizing,
prettifying, mode conversion, map/node finalization, inspect/map JSON output,
public SDK AES access, Starter Tonic, Titonic, and aeon-edit preserve all four
identity locations. Clone materialization clears identities copied from the
source subtree while retaining the destination binding identity. Semantic JSON
finalization intentionally omits identities and is documented as a lossy
materialization. The TypeScript and Tonics full typecheck/test baselines pass;
portable flat-event projection is implemented in all four language adapters,
while the wider consumer audit remains open.

### 1.2 Node projection and paths

#### Confirmed baseline

- [x] Represent a node as a value-less ordered `NodeLiteral` container.
- [x] Represent each node head as an indexed `NodeHead` event carrying its tag
  in `value`.
- [x] Current AEON produces one head at `[0]`; its content begins below that
  head at `[0][0]`, `[0][1]`, and so on.
- [x] Keep AES structurally capable of representing zero or multiple heads so
  future node forms do not require another event-model change.
- [x] Keep binding-head metadata on the outer `NodeLiteral` event and node-head
  metadata on the indexed `NodeHead` event.

#### Required ecosystem work

- [x] Update TypeScript, Rust, Python, and PHP AEON-to-AES adapters to expand a
  `NodeLiteral` into an outer `NodeLiteral`, a `NodeHead`, and recursively flattened
  content events.
  - [x] TypeScript: added the explicit portable projection, recursive node-path
    expansion, node-boundary reference translation, and CLI/CTS coverage.
  - [x] Rust: added the explicit portable projection, recursive node-path
    expansion, node-boundary reference translation, and CLI/CTS coverage.
  - [x] Python: added the explicit portable projection, recursive node-path
    expansion, node-boundary reference translation, and CLI/CTS coverage.
  - [x] PHP: added the explicit portable projection, recursive node-path
    expansion, node-boundary reference translation, and CLI/CTS coverage while
    retaining the existing event shape behind an explicit legacy JSON adapter.
- [x] Define AEON source-path to portable event-path translation for node
  descendants; the old first child path must never be silently reinterpreted
  as the new node-head path.
- [x] Define translation and validation for `CloneReference` and
  `PointerReference` target payloads across the additional node path level.
- [ ] Implement structure-aware source-path/event-path translation in all four
  language adapters, including reverse-projection rejection for direct
  synthetic node-head reference targets.
- [ ] Apply the same mapping to SANSA selections, Tonics edit addresses, SO/ASP
  mutation targets, diagnostics, and any public SDK navigation APIs.
- [x] Define the node-head source span as the tag token through the last
  identity, attribute, or datatype component, excluding node delimiters and
  children.
- [ ] Give the node tag/head its own source span in ASTs that currently expose
  only the complete node-literal span; emit origin-only provenance until that
  exact range is available.
- [x] Use the normative AEON representation-kind names at portable boundaries,
  including `StringLiteral`, `NodeLiteral`, and the new `NodeHead`; do not
  maintain a parallel lowercase or kebab-case vocabulary. The JavaScript and
  Rust validators enforce the shared
  vocabulary and value-presence rules; a shared negative conformance vector
  locks both sides of the `NodeLiteral`/`NodeHead` distinction.
- [x] Update SANSA structural navigation and parent/container compatibility for
  `NodeLiteral[head-index][content-index]`. The host-neutral resolver retains
  the adapter's explicit node -> node head -> content hierarchy, all four
  language parser surfaces accept the portable `%NodeHead` filter, and experimental
  SANSA Resolve snapshot 0.2 locks expansion, parent, attribute, nested-node,
  and legacy collapsed-path behavior.
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
- [x] Replace embedded attribute maps in TypeScript, Rust, Python, and PHP AES
  adapters and transport surfaces with flat projection or an explicit legacy
  compatibility adapter.
  - [x] TypeScript: the explicit portable projection emits binding,
    anonymous-child, nested, and node-head attributes as ordinary flat events.
  - [x] Rust: the explicit portable projection emits binding, anonymous-child,
    nested, and node-head attributes as ordinary flat events in source preorder.
  - [x] Python: the explicit portable projection emits binding,
    anonymous-child, nested, and node-head attributes as ordinary flat events
    in source preorder.
  - [x] PHP: the explicit portable projection emits binding, anonymous-child,
    nested, and node-head attributes as ordinary flat events in source preorder;
    the inspect transport keeps the prior nested form as an explicit legacy
    compatibility adapter.
- [ ] Verify prefix-completeness and container-compatibility rules for
  attribute paths without synthesizing phantom parent bindings.
- [ ] Update SANSA, SO, ASP, AES-DB, validators, SDKs, and editing tools that
  currently expect attributes to be nested inside a parent event.

### 1.4 Datatypes and values

- [x] Split the logical datatype into base-name `datatype`, recursive ordered
  `generics`, and ordered tagged-literal `clarifiers`; retain numeric argument
  payloads as strings and preserve duplicates.
- [x] Keep Telex compact by combining those fields into one canonical
  `datatype=` line, expanding on decode and recombining on encode.
- [x] Define generic depth as a shared event-local resource counter, independent
  of complete and partial semantic profiles; consumer-selected processing
  policy supplies the numeric limit and exhaustion never truncates a descriptor.
- [x] Keep a datatype only on the event where it was declared; do not infer or
  propagate container generic arguments onto descendants.
- [x] Use separate `CloneReference` and `PointerReference` kinds with one
  canonical target-path payload shape.
- [x] Under `aes.complete.v0`, require each reference target to exist exactly
  once in the body plane; under `aes.partial.v0`, validate target syntax without
  requiring local presence. Reference cycles remain a downstream concern.
- [x] Normalize trimtick content before AES and transport it as
  `kind=StringLiteral`;
  delimiter width and indentation are source mechanics.
- [x] Add `WTCDateTimeLiteral` as a distinct temporal kind.
- [x] Use exact lowercase `local` for the reserved WTC resolver-local
  reference.
- [x] Treat `conflictAuthority` as a consumer responsibility, not document or
  AES event authority.
- [x] Do not transport exact AEON lexemes or a generic representation field.
- [ ] Reconcile ASP, AEOS, and downstream CTS canonical payload rules with the
  portable table. TypeScript, Rust, Python, PHP, AES validators, and portable
  CTS now use the normative AEON representation-kind names.
- [x] Add WTC cases covering the three temporal anchor forms and local, named,
  and geographic references without introducing `conflictAuthority` into the
  portable event contract.
- [ ] Verify canonical payloads and semantic hashes preserve recognized value
  distinctions, including temporal distinctions, while excluding source-only
  spelling.
- [ ] Ensure portable boundaries use the normative AEON representation
  vocabulary without depending on runtime class identity, raw tokens, or
  nested value trees.

### 1.5 AltoPelago processing limits

- [x] Define the informative `altopelago.aeonic-limits.v1` file shape with an
  immutable `limits_id` and `limits_version`.
- [x] Keep structural counters unified across AEON, AES, Telex, Film, and
  direct in-memory AES ingress.
- [x] Keep byte, line, frame, and buffering limits scoped to their physical
  format or transport.
- [x] Treat `profile_claims` only as descriptive metadata; consumers select
  limits and semantic profiles independently.
- [x] Exclude AEOS schema-validation and SANSA evaluation budgets from this
  limits contract.
- [ ] Complete an implementation audit for public and hard-coded resource
  guards in TypeScript, Rust, Python, and PHP.
- [x] Approve and publish the first concrete AltoPelago `1.0.0` limits values
  and the fixed bootstrap policy used to load an AEON-encoded limits file.
- [ ] Add a shared limits loader, normalized effective-configuration view, and
  deterministic exhaustion diagnostics to every AltoPelago implementation.
  TypeScript, Rust, and Python now implement the AEON loader/compiler subset;
  PHP and non-AEON ingress remain to be completed.
- [x] Rename or adapt `maxSeparatorDepth` / `max_separator_depth` to the shared
  `max_clarifier_values` counter without creating a second semantic limit.
  TypeScript, Rust, and Python retain the former names only as migration aliases.
- [x] Add `max_generic_arguments` and `max_datatype_components` to AEON Core;
  keep `max_generic_depth` limited to recursive datatype depth.
- [x] Add the AEON v1 portability-floor counters for decoded string length, key
  segment length, numeric-literal lexical length, list and tuple length,
  canonical/reference path length, and structured-comment payload length.
- [x] Remove the TypeScript and Python canonicalizers' hard-coded generic and
  clarifier limit values; use the effective consumer-selected limits.
- [ ] Implement all required Telex bounds: input bytes, line bytes, fields per
  event, event count, decoded payload bytes, path depth, generic depth, and
  datatype component count.
- [ ] Add shared at-limit and one-over-limit vectors for every published
  counter.

### 1.6 Spans and provenance

#### Confirmed direction

- [x] Use zero-based UTF-8 byte offsets with an inclusive start and exclusive
  end.
- [x] Transport `span=start:end`; do not transport derived line and column
  coordinates.
- [x] Omit spans for events without source evidence and never fabricate a zero
  span.
- [x] Keep both fields optional: omit both without source evidence, permit
  `origin` alone when the source is known but the location is not, and require
  `origin` whenever `span` is present.
- [x] Use a record-local `sha256:<64 lowercase hex>` origin in Draft 0 so a
  stream can combine multiple sources without a source table. Film may compress
  repeated origins without changing the AES model.

#### Required ecosystem work

- [x] Define the portable source/origin identity contract. The SHA-256 digest
  covers the exact, unnormalized source byte sequence.
- [x] Define that offsets include an accepted UTF-8 BOM and always measure
  against the exact, unnormalized source resource.
- [x] Add shared Telex vectors for origin-only records, valid origin-plus-span,
  rejected span-only records, malformed and zero-width spans, and non-canonical
  origins.
- [x] Define UTF-8 scalar-boundary and source-length checks as source-backed
  audit rules rather than event-local validation; digest disagreement uses
  `AES_ORIGIN_MISMATCH`.
- [ ] Implement source-backed provenance audits over exact source artifacts in
  each language and add non-ASCII boundary, range, and digest-mismatch vectors.
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

### 1.7 AEON headers and control-plane metadata

- [x] Inventory how TypeScript, Rust, Python, and PHP currently expose
  `aeon:header` and shorthand `aeon:*` fields through AES. TypeScript and PHP
  synthesize public `aeon:*` events; Rust and Python keep parsed header metadata
  separately and omit it from their public body-event results.
- [x] Use body-only AEON-to-AES projection by default. The explicit
  `aeon.document.v0` projection adds flat records addressed with
  `header=$.["aeon:..."]`; a record has exactly one of `header` and `path`.
- [x] Keep `profile` and `projection` independent. Header records precede body
  events, have independent address uniqueness and completeness, and remain
  complete even when the body profile is partial. Body hashes/signatures omit
  them; explicitly scoped document hashes/signatures include both ordered
  planes. This is semantic header preservation, not exact AEON source
  round-tripping.
- [x] Add shared Telex vectors for explicit projection, body-only rejection,
  header ordering, mutually exclusive address fields, and independent nested
  header completeness.
- [ ] Add shared fixtures for structured headers, shorthand headers, header
  conflicts, and body-only streams.
- [x] TypeScript, Rust, and Python: `inspect --json --portable-aes` now exposes
  the portable projection as a body-only event stream. TypeScript excludes its
  synthetic `aeon:*` events; Rust and Python retain parsed headers outside the
  event stream.
- [ ] TypeScript, Rust, and Python: add an explicit `aeon.document.v0` adapter
  that projects the retained header into ordered `header` records before the
  portable body events.
- [ ] Audit remaining TypeScript public AES surfaces for synthetic header
  leakage, and update PHP's default body stream and explicit document adapter.
- [ ] Update finalizers and SDKs so existing payload/header/full views consume
  the two planes deliberately rather than filtering records by key prefix.
- [ ] Correct the AEON integrity appendix statement that convention headers are
  body state; it must distinguish body semantic coverage from explicit document
  coverage.

## 2. Repository and component work

### Specifications and conformance

- [x] Audit current specification ownership and identify AES material that must
  move, remain with a source language or consumer, or be reconciled. See
  `notes/specification-ownership-audit.md`.
- [x] `aeonite-specs`: establish the first-class `aes/v0` family, publish the
  transport-neutral event, Telex, compatibility, and semantic-language
  documents, and supersede the implementation-shaped AES appendix.
- [x] `aeonite-website`: add a first-class AES v0 section, family routing,
  home-page entry, sitemap/LLM discovery, and publication checks.
- [ ] `aeonite-specs`: update AEON node, structural-identity, span, reference,
  datatype, and WTC projection requirements.
- [ ] `aeonite-cts`: replace contradictory identity vectors and add portable
  event-local, complete-stream, path, value, and provenance suites.
- [ ] `aeonite-cts`: require at least two independent implementations to pass
  each portable contract before promotion.

### Language implementations and public surfaces

- [ ] `altopelago/aeon`: update TypeScript, Rust, and Python parsers, ASTs,
  flatteners, materializers, SDKs, CLIs, and JSON/debug projections.
- [ ] `altopelago/aeon`: update the existing TypeScript, Rust, and Python
  portable AES projections so they emit base-name `datatype`, recursive
  `generics`, and tagged `clarifiers` instead of collapsing the AST annotation
  back into one descriptor string.
- [ ] `altopelago/aeon-php`: implement the same portable contract and shared
  CTS coverage rather than treating PHP as a later compatibility exercise.
- [ ] `altopelago/aeon-php`: expose the same recursive datatype structure when
  its portable projection is added; never decode numeric arguments through a
  PHP numeric type.
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
- [ ] ASP: preserve kind, canonical value, all three datatype components,
  identity, and provenance without rebuilding source lexemes.
- [ ] ASP: update operations and origin handling for expanded paths and
  source-backed spans.
- [ ] AES-DB: reconstruct value-less containers and explicit descendants while
  remaining neutral on datatype inference, reference resolution, and
  value-family semantics.
- [ ] Audit relays, canonicalizers, signing paths, and durable codecs for
  assumptions about the legacy event shape.
- [ ] Update semantic hashes and signatures to bind expanded datatype structure
  rather than a Telex-specific combined descriptor string.

## 3. Compatibility and stored-data migration

- [x] Define a versioned compatibility projection from legacy
  legacy `kind=node,value=tag` records to an outer `NodeLiteral` plus indexed
  `NodeHead`.
- [x] Define compatibility behavior for stored events created before stable
  structural identity was required: preserve a present identity and leave a
  missing one absent rather than manufacturing it.
- [x] Preserve legacy numeric node-child meaning; do not reinterpret it as a
  node-head path without an explicit versioned projection.
- [x] Keep durable history in its original contract by default and expose a
  versioned read-time compatibility view. An explicitly authorized migration
  writes a new versioned store after backup, replay, index, and restore proof.
- [ ] Implement named, versioned legacy-to-portable adapters and conversion
  reports for every supported implementation-specific source contract.
- [ ] Rebuild or invalidate AES-DB path, datatype, attribute, reference, and
  ordered-child indexes affected by the revised projection.
- [ ] Version persisted ASP/AES-DB records whose span units or checksummed
  source bytes change; do not silently rewrite historical logs.
- [ ] Verify subtree moves, deletion, replay, snapshots, checkpoints, backup,
  restore, and compaction preserve node-head and identity occurrences.
- [x] Define semantic losslessness relative to the selected AES profile and
  projection, separately from full record/provenance fidelity. Exact source
  reconstruction requires the separately retained artifact identified by
  provenance and is not implied by a semantic event round trip.
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
- [ ] Compatibility context: untagged JSON is rejected at portable boundaries;
  every non-Telex carrier binds records explicitly to `aes.events.v0`.

## 5. Rollout sequence

- [ ] Phase 1 — freeze and publish the transport-neutral `aes.events.v0`
  contract and its version discriminator.
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

The following belongs to the Telex encoding specification and repository-local
work, not this ecosystem sweep:

- Wire grammar, separators, escaping, BOM handling, and exact serialization.
- Telex carriage of AES profile and projection identifiers, stream headers,
  and codec versioning.
- Telex syntax-layer handling of unknown and extension fields.
- JavaScript/Rust Telex codec parity and Draft 0 implementation milestones.
- Telex fixture promotion except where a fixture becomes a transport-neutral
  AES or shared CTS case.
