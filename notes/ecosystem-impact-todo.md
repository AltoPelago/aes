# AES ecosystem impact checklist

Status: living working list

Updated: 2026-09-07

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
- [x] Land shared CTS coverage for the agreed contract before implementations
  claim support. `aes-events-cts-v0-snapshot-0.1` freezes 38 transport-neutral
  validation vectors and `telex-cts-v0-snapshot-0.1` freezes 50 encoding and
  format-limit vectors, each with per-suite SHA-256 digests. JavaScript and
  Rust pass both published targets independently; the AEON-to-AES projection
  candidate passes 67/67 in TypeScript, Rust, Python, and PHP.
- [x] Define reader-first compatibility rules before any producer emits the
  revised shape into durable stores or cross-service interfaces.

### Latest contract consistency audit

The 2026-09-05 audit extracted transport-neutral semantics into
`specifications/aes.events.md` and reduced the Telex document to encoding
concerns. Conformance metadata now points each suite at its owning specification.
Repository tests verify that every referenced specification heading resolves
and that the documented core fields, value kinds, and semantic diagnostic codes
remain aligned with the JavaScript reference validator.

All contract-definition and shared-CTS release gates are now complete.
Reader-first conversion, persistence, capability, and writer-activation rules
are defined in `specifications/aes.compatibility.md`; their ecosystem
implementation and acceptance work remains open below.

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
- [x] Implement structure-aware source-path/event-path translation in all four
  language adapters, including reverse-projection rejection for direct
  synthetic node-head reference targets. The experimental
  `aes-path-translation-cts-v0-snapshot-0.1` target passes 3/3 in TypeScript,
  Rust, Python, and PHP. It covers recursive head-index insertion,
  clone/pointer retargeting, reverse materialization of node-content references,
  and proof that `$.a[0]` remains the synthetic head rather than the legacy
  first child.
- [x] Apply the same mapping to every public consumer surface.
  - [x] SANSA selection/navigation and SO/ASP mutation targets use the expanded
    structural mapping without making identity part of path identity.
  - [x] The Tonics audit records the mapping, and `aes-diff` operates on
    validated complete portable streams.
  - [x] Tonics `aeon-edit` lists and accepts portable node-head/content paths,
    requires an explicit `NodeHead` path for node metadata operations, and
    fails closed rather than reinterpreting that path as a child. Titonic SANSA
    navigation exposes portable `pathText`/`portablePath` while retaining its
    child marker only as the explicitly named `titonicPath`. Graph, search, and
    lint paths, reference targets, and diagnostic path fields use the portable
    projection.
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
- [x] Update AEOS datatype and cardinality validation for the AEON requirement
  of exactly one node head. Complete Telex validation establishes the single
  `[0]` head invariant before AEOS applies schema rules, while AEOS cardinality
  counts the indexed content beneath that head.
- [x] Update Tonics and other editing tools that address node-head metadata
  through node-specific commands or legacy child paths. `aeon-edit` node-head
  metadata commands now require `$.node[0]`; listed node content begins at
  `$.node[0][0]`, and outer-node addresses are rejected for those commands.

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
  - [x] TypeScript, Rust, and Python AEON parsing/compilation counters are
    mapped to the normalized limits file and exercised through shared vectors.
  - [x] PHP is inventoried in `php-aeonic-limits-audit.md`; its closed loader,
    canonical names, and all 16 AEON compiler counters are implemented. The
    complete consolidated Core-next protocol target passes 265/265; future
    finalization, Telex, and transport surfaces remain open.
  - [ ] Non-AEON ingress still requires the same audit.
- [x] Approve and publish the first concrete AltoPelago `1.0.0` limits values
  and the fixed bootstrap policy used to load an AEON-encoded limits file.
- [ ] Add a shared limits loader, normalized effective-configuration view, and
  deterministic exhaustion diagnostics to every AltoPelago implementation.
  TypeScript, Rust, Python, and PHP now implement the AEON compiler subset;
  TypeScript, Rust, and Python also implement finalization, while TypeScript
  exposes normalized framing values. Other direct AES ingress remains open.
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
- [x] Implement all required Telex bounds: input bytes, line bytes, fields per
  event, event count, decoded payload bytes, path depth, generic depth, and
  datatype component count. The JavaScript and Rust reference codecs consume
  normalized effective limits; limits-file loading remains a trusted caller concern.
- [ ] Add shared at-limit and one-over-limit vectors for every published
  counter. Telex v0, all 16 AEON parsing/compilation counters, reference
  resolution, materialization, and transport framing are covered by
  language-neutral suites; future Film counters remain.

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
  - [x] The ASP-hosted SO projection emits provenance only when `kind=source`
    and `source_id` is a portable SHA-256 origin; it never emits an ASP span
    without that origin. Orchestrator, migration, and API origins remain
    internal even when their IDs happen to look like content digests.
  - [ ] Define storage/operation semantics for retaining the exact source
    artifact contract rather than inferring it from the existing ASP fields.
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
  - [x] TypeScript: `compileToTelex()`/`exportTelex()` and CLI
    `inspect --telex --include-headers` emit ordered header-plane records before
    portable body records; body-only remains the default.
  - [x] Rust: Core/SDK export and CLI `inspect --telex --include-headers`
    preserve header binding order and keep body-only export as the default.
  - [x] Python: Core/API export and CLI `inspect --telex --include-headers`
    preserve header binding order and keep body-only export as the default.
- [ ] Audit remaining TypeScript public AES surfaces for synthetic header
  leakage, and update PHP's default body stream and explicit document adapter.
- [ ] Update finalizers and SDKs so existing payload/header/full views consume
  the two planes deliberately rather than filtering records by key prefix.
  - [x] TypeScript, Rust, and Python portable JSON materializers consume the
    explicit planes; their SDK and CLI surfaces expose payload/header/full
    scope deliberately.
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
  - [x] TypeScript: ship the Telex v0 codec through the AES package; add
    Core event/source export, SDK read/write, AEOS validation, canonicalization,
    and CLI decode/export surfaces while retaining the legacy in-memory APIs.
    The published AES Events and Telex snapshot suites pass 38/38 and 50/50.
  - [x] TypeScript: consume complete portable AES directly for semantic JSON
    materialization without rebuilding the parser AST. The finalizer, SDK,
    runtime schema pipeline, and CLI expose this path; differential tests cover
    nested containers, flat attributes, node heads, identities, datatype
    components, headers, clones, and symbolic pointers. Partial streams require
    external state and are rejected by this materializer.
  - [ ] TypeScript: add portable map/node output and live pointer-linking only
    when consumers require those output profiles. The current portable JSON
    path deliberately keeps pointers symbolic and does not run AEON source
    processors or tonics.
  - [x] Rust/WASM: reuse the AES-owned Rust reference codec from the AEON WASM
    package for bulk validation, canonicalization, and prefix-completeness
    checks. Keep full event materialization on the TypeScript surface until a
    low-copy record bridge is justified by benchmarks. A JSON-bridged WASM
    encoder was measured and rejected: for JavaScript-resident records it was
    27–45% slower than the optimized TypeScript encoder despite the native Rust
    encoder being about seven times faster than TypeScript at limit scale.
  - [x] Rust: expose Core event/source export, SDK Telex read/write, direct
    portable materialization, AEOS validation, and CLI decode/canonicalize/
    materialize/export while retaining the legacy in-memory APIs.
  - [x] Python: expose Core event/source export, API Telex read/write, direct
    portable materialization, AEOS validation, and CLI decode/canonicalize/
    materialize/export while retaining the legacy in-memory APIs. The shared
    AES Events and Telex snapshot suites pass 38/38 and 50/50; Python output is
    byte-identical to TypeScript and Rust on the full-feature stress fixture.
  - [ ] Promote `aes-telex` from its unpublished `0.0.0` reference-crate state
    to a versioned dependency before AEON and AES need independently releasable
    Rust/WASM build graphs. The family workspace currently uses an explicit
    sibling path dependency.
- [ ] `altopelago/aeon`: update the existing TypeScript, Rust, and Python
  portable AES projections so they emit base-name `datatype`, recursive
  `generics`, and tagged `clarifiers` instead of collapsing the AST annotation
  back into one descriptor string.
  - [x] TypeScript: the portable projection now exposes all three fields and
    Telex recombines them only at the wire boundary.
  - [x] Rust and Python: the portable projections now expose all three fields
    and Telex recombines them only at the wire boundary.
- [x] `altopelago/aeon-php`: implement the same portable contract and shared
  CTS coverage rather than treating PHP as a later compatibility exercise.
- [x] `altopelago/aeon-php`: expose the same recursive datatype structure in
  its portable projection; numeric arguments remain strings.
- [x] `altopelago/aeon-validator`: accept explicitly identified AEON or Telex
  snippets, apply Telex syntax and AES semantic validation, report
  completeness, and optionally materialize complete portable streams without
  changing the existing AEON default.
- [x] `altopelago/aeon-tooling`: inventory commands and interchange surfaces
  that consume or emit AES-shaped JSON. The audit found no production
  serialized AES boundary: VS Code/AEOS validation and Neon canonicalization
  use same-process objects. The standalone CTS runner's JSON is the
  `cts.protocol.v1` control envelope, not portable interchange; it now forwards
  explicit portable-AES cases and preserves structural identities while keeping
  Telex conformance in the Telex lane.
- [x] `altopelago/aeon-tonics`: audit canonical rendering, formatting,
  conversion, editing addresses, and node/attribute handling. Native
  `AssignmentEvent[]` remains the same-process API; legacy AES JSON routes are
  explicit compatibility surfaces. `aes-diff` now validates, compares,
  patches, and re-emits complete Telex, while `aeon-edit` exports complete
  Telex with document headers opt-in. The workspace audit records the
  `$.a`/`$.a[0]`/`$.a[0][0]` translation, attribute ownership, identity,
  datatype-component, header, and ordering contracts.

### Semantic, operational, and persistence consumers

- [ ] SANSA: update structural navigation, ownership, scope, reference
  resolution, and source-path/event-path translation.
  - [x] The read-only Query CLI and workbench accept complete Telex streams
    through a direct portable-event adapter. Record order, flat attribute
    spaces, datatype components, structural identities, and the explicit
    `NodeLiteral` → `NodeHead` → content hierarchy remain independent of path
    identity; partial streams fail closed without external namespace state.
  - [x] Extend the portable adapter deliberately to the first unambiguous
    mutation/editing surface. SANSA now accepts complete Telex directly for
    exact scalar replacement and re-emits complete Telex without AEON source
    reconstruction. It preserves event order, paths, identities, datatype
    components, and flat attributes; the changed event drops stale
    `origin`/`span`. Create, remove, insert, move, container/node-head changes,
    and reference rewrites remain rejected until the portable path-rewrite and
    reference-translation contract is specified.
- [x] AEOS: update datatype, cardinality, kind, node-head, and WTC validation.
  - [x] TypeScript, Rust, and Python accept complete Telex directly, preserve
    normative representation kinds, reconstruct split datatype components for
    schema comparison, and validate flat attributes without adding structural
    identity to path identity.
  - [x] Explicit adapter vectors cover `NodeLiteral`/`NodeHead` paths and
    content cardinality. The shared AES Events and Telex snapshot vectors cover
    the complete WTC anchor/reference matrix, lexical preservation, and exact
    lowercase `local`.
  - [x] The rollout audit fixed detached portable attributes in TypeScript, an
    empty-stream normalization bug in Python AEOS Telex validation, and string
    rather than numeric portable indexes in the Rust AEOS adapter.
- [x] SO: add an explicit portable-AES adapter instead of treating current
  TypeScript AST-shaped values as the interchange contract. The ASP target
  adapter now exposes direct complete portable records, Telex encoding, and
  AEOS validators for both forms while retaining the legacy AST-shaped API as
  an explicit compatibility path.

#### Consolidated SO/ASP mutation status

This is the primary status view for the SO/ASP work. Supporting an AES event in
Telex, projection, validation, and read/export paths does not imply that ASP v0
can mutate that event as an independent storage occurrence. Telex/AES
compatibility is complete for a form when it is preserved or rejected through
an explicit, versioned boundary; it does not require every representable AES
form to become an ASP mutation target.

The former broad task to "complete source/event address translation and
application semantics for expanded node-head and non-scalar inline-descendant
mutation" is retired. Its completed portion and exact remaining inventory are
recorded below.

##### Complete

- [x] **Portable addressing foundation:** construct and validate portable
  candidates with flat attributes, expanded nodes, split datatype components,
  stable order, and identity independent of path; translate admitted source
  and event addresses bidirectionally under an immutable source revision.
- [x] **Scalar replacement:** apply direct and contained scalar replacement,
  including tuple items, node children, node-head attributes, and nested
  binding attributes.
- [x] **Reference retargeting:** translate clone/pointer targets reversibly and
  apply direct or one-owner contained retargeting.
- [x] **Tuple-content replacement:** replace a closed tuple fragment, including
  nested tuples and fragment-local references, while retaining root occurrence
  metadata.
- [x] **Flat material-content replacement:** replace ordered direct
  scalar/reference children of an object or list through an exact multi-owner
  ASP transaction.
- [x] **Recursive material-content replacement:** replace nested object/list
  owners and inline tuple content, preserve child binding-owner identity and
  the complete datatype component triple, and enforce the selected value-
  nesting limit.
- [x] **Operational closure for the admitted applications:** scalar,
  reference, tuple, flat-material, and recursive-material applications have
  explicit SO dispatch, atomic revision preconditions, durable fenced receipts,
  restart reconciliation, and fail-closed candidate/result validation.
- [x] **Cross-language path translation acceptance:** TypeScript, Rust, Python,
  and PHP pass the same versioned forward/reverse vectors for recursive node
  boundaries and references; a direct synthetic `NodeHead` target fails closed
  without being reinterpreted as a legacy child.
- [x] **Public consumer path propagation:** TypeScript exposes a reusable
  native-to-portable event path map; Tonics edit, Titonic SANSA navigation,
  graph, search, lint, and compile-diagnostic surfaces preserve the expanded
  node-head hierarchy and translated reference targets.

##### Remaining Telex/AES compatibility requirements

- [x] Complete named legacy-to-portable adapters and conversion reports for
  every supported implementation-specific source contract. TypeScript, Rust,
  Python, PHP, and ASP now expose implementation-owned v0 source identifiers,
  named adapters, strict `aes.complete.v0` results, and reports that separate
  semantic, record, and provenance fidelity. Local spans without an immutable
  origin are omitted and reported; existing same-process projection helpers
  remain available but do not identify an interchange contract.
- [ ] Complete the AES-DB portable projection for value-less containers and
  explicit descendants while keeping datatype inference, reference resolution,
  and value-family semantics in their owning layers. The existing ASP-backed
  strict read view is evidence for this task, not closure of every AES-DB
  storage/projection path.
- [ ] Audit relays, canonicalizers, signing paths, and durable codecs for the
  legacy event shape, and bind semantic hashes/signatures to
  `aes.events.v0`, the effective profile/projection, ordering policy, and the
  expanded datatype structure rather than a Telex descriptor string.
- [ ] Run the existing cross-repository acceptance set for identities, nodes,
  paths/references, attributes, values, headers, provenance, compatibility,
  and AEON -> portable AES/Telex -> SO -> ASP -> AES-DB -> portable AES/Telex.
  A consumer may pass an unsupported-mutation vector by returning the specified
  stable rejection without side effects.

##### Remaining production-safe SO/ASP requirements

- [ ] Define and register a versioned application/transaction carrier before
  any public actionable ingress is enabled. It must bind the selected existing
  application contract, target, preconditions, ordered AES payload, assertions,
  effective limits, authorization context, and integrity policy. This closes
  an existing AET/SO boundary requirement; it does not make a bare Telex stream
  actionable and does not add another mutation family.
- [ ] Complete origin/span operation semantics against an exact retained source
  artifact when a selected application claims source-backed provenance.
  Changed records may continue to drop stale provenance, and applications that
  make no retention claim may continue to omit it.
- [ ] Rebuild or invalidate affected AES-DB indexes and verify replay,
  snapshots, checkpoints, backup, restore, compaction, subtree deletion/move,
  and mixed legacy/revised reads preserve the admitted portable occurrences.
- [ ] Define separate canonical-semantic and exact-order ledger signature
  policies, including deterministic logical bytes and version/profile/order
  binding, before signatures are used as independently verifiable application
  evidence.

##### Blocked by a versioned ASP storage/writer contract

- [ ] **Node-head mutation:** direct `NodeHead` tag or occurrence-owned
  identity, datatype, attribute-set, and provenance mutation is blocked. ASP v0
  stores one implicit tag and synthesizes the portable `[0]` head; it must not
  be widened. Existing scalar replacement of a node-head attribute descendant
  remains admitted. Resume head mutation only after explicit approval of a
  versioned head-aware record and writer contract with mixed-history vectors.
- [ ] **Node-literal/subtree replacement:** replacing a `NodeLiteral`, nesting
  a node in a replacement fragment, or representing zero/multiple heads is
  blocked by the same head-aware storage dependency.
- [ ] **Independently annotated inline descendants:** identity, datatype,
  attributes, or provenance on an inline tuple-item occurrence is blocked
  because ASP v0 has no independent metadata owner for that occurrence.
- [ ] **Material ownership inside tuples:** object/list binding owners nested
  inside an inline tuple are blocked until a versioned storage layout can
  preserve those owners and their addresses without changing ASP v0 history.
- [ ] **Native split datatype persistence:** ASP v0 may continue its named,
  loss-accounted compatibility projection from one canonical combined
  descriptor. Persisting `datatype`, `generics`, and `clarifiers` independently
  requires a versioned record/writer contract.
- [ ] **Revised span/source storage:** any change to persisted span units or
  checksummed source-artifact identity requires versioned records and explicit
  migration/read-view behavior; historical logs must not be silently rewritten.

##### Optional future mutation capabilities

- [ ] Admit recursive attribute trees on inserted/replaced material child
  owners; they currently fail with
  `AES_COMPAT_MATERIAL_ATTRIBUTES_UNSUPPORTED`.
- [ ] Define migration or retention semantics for descendant contracts during
  material replacement; candidates containing them currently fail closed.
- [ ] Add root occurrence-metadata replacement to tuple/material applications;
  the current content-replacement contracts deliberately retain target-owned
  root identity, datatype, attributes, provenance, and contract.
- [ ] Add create, remove, insert, move, merge, or general subtree operations.
  The current named application set is replacement/retarget-only.
- [ ] Generalize beyond the first registered application-specific carrier, add
  Wire/CLI mutation routes, or add a self-contained Telex transaction mapping
  after the versioned carrier and security decisions are approved.

##### Deliberately out of scope

- [x] A bare `telex.aes=0` stream is never mutation authority. Telex import and
  export remain non-actionable; trusted host policy and a versioned application
  envelope select an operation.
- [x] Structural identity does not become path identity, and transported
  `origin`/`span` does not become authorization, ordering, or mutation identity.
- [x] AEON signature/encryption conventions remain a security envelope around
  covered application material, not fields inferred from ordinary AES events.
- [x] ASP v0 historical records and writers are not silently extended to make
  deferred forms appear supported.

<details>
<summary>Implementation record: completed SO/ASP mutation milestones (A.18-A.38)</summary>

- [x] SO: update candidate construction, validation, stable-order scopes, and
  path-addressed operations for the currently admitted flat-attribute,
  expanded-node descendant, and material-content replacement forms.
  - [x] ASP-hosted candidate construction and validation emit flat attributes,
    expanded node paths, split datatype components, and independently retained
    identities. Existing visible stable-order projections feed the same adapter
    and preserve the identity-to-index association and record order.
  - [x] Complete source/event address translation and application semantics for
    the admitted scalar, reference, tuple, flat-material, and recursive-material
    targets. Unsupported node-head and inline-owner forms remain explicitly
    classified in the primary checklist above.
    - [x] ASP reader-side prerequisite: a named read-context path index maps
      explicit structural source routes bidirectionally to portable event
      paths, covers recursively expanded node heads, and rejects ambiguous
      legacy attribute-reference spellings. Portable projection verifies its
      path sequence against the same index. Identity remains metadata and the
      index alone grants no mutation authority.
    - [x] ASP mutation-target preflight: consume the reverse index with exact
      source-revision and portable-kind expectations. Existing scalar bindings
      and direct binding attributes are storage-native candidates; scalar tuple
      items, node children, node-head attributes, and nested binding attributes
      are contained candidates through one exact storage owner. Containers,
      references, heads, non-scalar descendants, other operations, and
      stale/substituted context fail closed. Successful preflight remains
      non-actionable, names no application contract, and produces no ASP
      operation or transaction.
    - [x] First named ASP application candidate: the closed
      `aes.application.asp.scalar-replacement.v0-candidate` request admits only
      value replacement of one existing scalar occurrence. Direct targets map
      to their native ASP operation; contained tuple items, node children,
      node-head attributes, and nested binding attributes rebuild exactly one
      existing binding or direct attribute. It preserves kind, combined
      datatype, structural identity, siblings, nested attributes, and order;
      drops stale source lexeme/provenance on changed records; and
      requires distinct intent/attempt/ASP transaction identities, request and
      resolved-plan authorization, exact candidate validation, and an atomic
      root-revision precondition. Unknown fields fail before authorization.
      Telex and bare AES streams remain non-actionable, while generic
      transaction carrier, integrity, encryption, and profile-limit gates remain
      open. Local unit and ASP conformance vectors cover the candidate without
      making a public AES transaction conformance claim.
    - [x] Contained scalar lowering: portable source routes now select one exact
      ASP storage owner and replace only the scalar leaf within its cloned value
      or attribute tree. The application emits one atomic `put_assignment` or
      `put_attribute`, distinguishes contained from native capability, and is
      covered by fixed node-child, node-head-attribute, tuple-item, and nested
      attribute vectors plus durable SO dispatch. Node-head, reference,
      container, non-scalar, and multi-owner structural mutation remain closed.
    - [x] Node-head replacement decision: do not register an application on ASP
      v0. Portable heads may own identity, datatype components, attributes,
      origin/span, and independent cardinality, while ASP v0 stores only one
      implicit tag and synthesizes `[0]` on export. Preflight now returns
      `AES_COMPAT_NODE_HEAD_STORAGE_REQUIRED`; SO classifies it as an unsupported
      target without candidate, journal, or commit effects. A future application
      requires a versioned head-aware ASP record/writer contract and
      mixed-history vectors first.
    - [x] Reference payload translation prerequisite: portable reference targets
      now reverse through the same occurrence index to an ASP path only when
      round-tripping selects the identical non-head occurrence. Synthetic
      `NodeHead` targets and portable targets that collapse to an ambiguous
      legacy attribute spelling fail closed. A closed clone/pointer retarget
      preflight preserves reference kind, binds the exact revision and proposed
      target, and identifies one storage owner, but remains non-actionable with
      no application contract, operation, transaction, or receipt.
    - [x] Named reference-retarget application candidate: the closed
      `aes.application.asp.reference-retarget.v0-candidate` request selects one
      existing clone or pointer occurrence and changes only its target payload.
      It reverse-translates the portable target, validates the complete
      post-retarget event sequence and exact prepared ASP candidate, preserves
      reference kind and surrounding metadata, and reconstructs at most one
      binding or direct attribute under request and resolved-plan authorization
      plus an atomic root-revision precondition. The direct helper returns only
      process-local commit evidence; SO dispatch, Telex/Wire ingress, and
      generic AET authority remain closed.
    - [x] Durable reference-retarget receipt: a separate domain-separated,
      fsync-backed, fenced single-writer journal retains the exact authorized
      plan before commit and binds database, intent, attempt, ASP transaction,
      request/application-context/plan/transaction fingerprints, and adjacent
      revisions in a deterministic receipt. Restart uses exact target
      transaction lookup to reconcile completion loss. Identity collision,
      request/plan detachment, target divergence, transaction collision,
      volatile targets, writer displacement, and receipt tampering fail closed.
      The receipt is neither independent policy evidence nor an AEON security
      envelope; public ingress remains closed.
    - [x] Explicit reference-retarget SO dispatch: the shared closed application
      envelope now distinguishes scalar replacement values from reference
      replacement target paths, and each adapter rejects the other's shape
      before callbacks. The reference profile negotiates the exact application,
      ASP substrate and database, reversible target translation, native and
      contained owner reconstruction, candidate preparation, atomic revision,
      durable commit/lookup, and receipt journal capabilities. Direct and SO
      execution return identical durable receipt bytes. Trusted origin remains
      host context, `telexIngress` is false, and Wire/CLI/bare-event/generic AET
      ingress remain closed.
    - [x] Structural replacement topology preflight: direct and contained
      `TupleLiteral` subtrees are classified as ordered one-owner inline
      candidates and report their exact source route and current event paths,
      but remain non-actionable with no replacement or transaction.
      `NodeLiteral`/`NodeHead` remain behind head-aware storage;
      storage-native `ObjectNode`/`ListNode` roots are now classified as
      multi-owner material subtrees and report their exact ordered event paths,
      owner count, and binding paths. All topology results remain non-actionable.
    - [x] Tuple-content inverse prerequisite: a closed preorder fragment rooted
      at `TupleLiteral` reconstructs nested tuple/scalar/reference ASP values,
      enforces exact boundaries and contiguous indices, and resolves portable
      reference targets outside the replacement against the immutable read.
      Fragment-local reference targets must identify exactly one event in the
      closed replacement and translate by their indexed suffix relative to the
      selected tuple owner's legacy path; this preserves contained node-head
      shifts without using structural identity as path identity. Missing or
      non-indexed local targets fail closed. Mandatory separate payload and
      result direct-item counts both match the decoded arity for this full-
      replacement slice, including explicit zero for empty tuples. Existing
      occurrence metadata/attributes remain target-owned; inline identity,
      datatype components, provenance/span, attributes, nested nodes, and
      material object/list containers fail closed. The result is semantically
      lossless for the admitted slice but has no application, operation,
      transaction, receipt, or SO authority.
    - [x] Named tuple-content replacement application candidate: the closed
      `aes.application.asp.tuple-content-replacement.v0-candidate` request joins
      the one-owner tuple topology preflight with the pure tuple inverse. It
      retains the existing root identity, combined datatype, attributes, and
      order; validates both the complete post-replacement portable event view
      and exact prepared ASP candidate; and rewrites one binding or direct
      attribute under request and resolved-plan authorization plus an atomic
      root-revision precondition. Direct, binding-attribute, and contained node
      owner vectors cover the candidate. Transported provenance remains
      rejected, trusted origin is host context, and journal/SO/Telex/Wire/CLI
      and generic AET authority remain closed.
    - [x] Durable tuple-content replacement receipt: a separate domain-
      separated, fsync-backed, fenced single-writer journal retains the exact
      authorized tuple plan before commit. Its deterministic receipt binds the
      database, intent, attempt, ASP transaction, request/application-context/
      plan/exact-transaction fingerprints, and adjacent revisions. Restart
      reconciles completion loss through authoritative exact ASP transaction
      lookup; identity reuse, plan detachment, transaction collision, target
      divergence, volatile targets, writer displacement, and persisted receipt
      tampering fail closed. The sidecar remains implementation state rather
      than AES data, and SO/Telex/Wire/CLI/generic AET ingress remains closed.
    - [x] Explicit tuple-content SO dispatch: the shared closed application
      envelope now has a third mutually exclusive input shape carrying the
      portable tuple fragment and both direct-item counts. The ASP adapter
      negotiates the exact tuple application, bound database, closed fragment,
      complete result validation, portable inverse, reference translation,
      one-owner reconstruction, candidate preparation, atomic revision,
      durable commit/lookup, and receipt journal capabilities. Direct durable
      and SO execution return identical receipt bytes while phase-specific
      policy and validation failures remain visible. Trusted origin stays host
      context, `telexIngress` is false, and Wire/CLI/bare-event/generic AET
      ingress remain closed.
    - [x] Fragment-local tuple references: clone and pointer payloads may target
      the replacement tuple itself or any exactly present indexed occurrence in
      the same closed fragment. Translation is rooted in the selected immutable
      source occurrence, so tuples beneath a portable node head preserve the
      synthetic `[0]` shift while ASP stores the corresponding legacy path.
      Unit, application, and fixed conformance vectors cover direct and
      node-contained mappings. This adds no identity-based addressing, nested
      node/material-container support, Telex ingress, or broader mutation
      authority.
    - [x] Flat material-container inverse: a closed `ObjectNode` or `ListNode`
      fragment can reconstruct its ordered direct scalar/reference child
      bindings without mutation authority. Object member paths must be direct;
      list indices must be contiguous from zero; separate payload and result
      counts must match the child binding count. External targets resolve
      through the immutable read and local targets must identify an exact
      supplied root or child. Root metadata remains target-owned, while child
      metadata/attributes, nested material owners, tuples, and nodes fail closed
      pending richer reconstruction. The result has no application, operation,
      transaction, durable receipt, SO route, or Telex ingress.
    - [x] Named flat material-content replacement application: the closed
      `aes.application.asp.flat-material-content-replacement.v0-candidate`
      request joins exact multi-owner preflight with the flat inverse. It keeps
      the root binding and its metadata/attributes/provenance/root contracts,
      tombstones every current descendant binding deepest-first, and inserts
      the reconstructed direct children atomically under one root-revision
      precondition. Descendant contracts fail closed. Complete AES validation
      runs before candidate preparation, and exact candidate reprojection
      prevents ASP v0 canonical binding order from silently changing requested
      object event order. Request, candidate, and plan authorization remain
      separate; no-op, widened, stale, and denied requests publish nothing.
      The direct helper returns process-local evidence only; durability and SO
      dispatch are selected separately below, while Wire/CLI, Telex ingress,
      and generic AET authority remain closed.
    - [x] Candidate-specific flat material-content durable receipt: an
      fsync-backed, single-writer sidecar retains the authorized exact
      multi-owner plan—including the ordered tombstone and insertion set—before
      commit and binds database, intent, attempt, ASP transaction,
      request/application-context/plan/exact transaction fingerprints, and
      source/commit revisions in a deterministic receipt. Restart reconciles
      completion loss through exact durable ASP transaction lookup; context
      reuse, collisions, divergence, volatile targets, writer fencing, and
      tampering fail closed. The sidecar does not write into user paths, its
      10,000-entry/16-MiB bounds are implementation limits rather than AES
      claims, and its fingerprint is neither a signature nor an AEON security
      envelope. Generic AET receipts and independently verifiable authorization
      evidence remain open.
    - [x] Explicit flat material-content SO dispatch: the shared closed
      host-neutral envelope now has a fourth application input alternative.
      `replacementEvents` identifies structural content while `expectedKind`
      discriminates tuple content from the admitted `ObjectNode`/`ListNode`
      material roots. The ASP adapter binds the named application to one
      `asp.v0` substrate and database and negotiates complete fragment/result
      validation, flat inverse translation, exact multi-owner reconstruction,
      candidate preparation, atomic revision, durable commit/lookup, and
      receipt-journal capabilities. Request, candidate, and plan policy phases
      remain distinct. Direct durable and SO execution return identical receipt
      bytes. Fresh origin remains trusted host context; descendant contracts
      and richer material descendants fail closed. This remains separate from
      SANSA mutation and advertises no Telex, Wire, CLI, bare-event, or generic
      transaction ingress.
    - [x] Recursive material-container inverse: a new closed, non-actionable
      inverse reconstructs nested `ObjectNode`/`ListNode` binding owners and
      inline tuple values without widening the pinned flat candidate. Every
      container supplies separate payload/result direct-item counts in event
      preorder. Fragment-local references bind to one exact supplied occurrence;
      external references resolve through the immutable ASP read. Logical
      container recursion uses a consumer-selected `maxValueNestingDepth`, with
      root depth 1 and the common implementation default 256. Root metadata
      remains target-owned; child metadata was initially closed and is admitted
      only by the explicit owner-metadata gate below. Nodes and material
      ownership inside tuples fail closed. The result names no application contract and grants no
      deletion, insertion, transaction, durability, SO, Wire/CLI, bare-event,
      generic AET, or Telex-ingress authority.
    - [x] Named recursive material-content replacement application: the closed
      `aes.application.asp.recursive-material-content-replacement.v0-candidate`
      request joins exact multi-owner preflight with the recursive inverse. It
      retains the root binding and root-owned metadata, attributes, provenance,
      and contract; tombstones all current descendant owners deepest-first; and
      inserts the reconstructed nested owner set atomically under one root-
      revision precondition. Descendant contracts fail closed. The effective
      consumer-selected value-nesting limit is retained in the prepared plan.
      Complete AES validation and exact candidate reprojection prevent storage
      ordering from changing the supplied event sequence. Request, candidate,
      and plan authorization remain distinct, and stale/no-op/widened requests
      publish nothing. The direct result is process-local evidence only;
      durability, SO, Wire/CLI, bare-event, generic AET, and Telex ingress remain
      later gates.
    - [x] Candidate-specific recursive material-content durable receipt: an
      fsync-backed, fenced single-writer sidecar retains the exact authorized
      nested tombstone/insertion plan before commit. The recursive-specific
      deterministic receipt binds database, intent, attempt, ASP transaction,
      request, trusted application context, prepared plan, exact transaction,
      and adjacent revisions. The effective value-nesting limit is bound by
      both application-context and plan fingerprints. Restart reconciles
      completion loss through authoritative exact transaction lookup; identity
      reuse, context/plan detachment, collision, target divergence, volatile
      targets, writer displacement, and retained evidence tampering fail closed.
      Its 10,000-entry and 16-MiB bounds remain local implementation limits.
      The fingerprint is integrity evidence, not a signature, generic AET
      receipt, or AEON security envelope; SO, Wire/CLI, bare-event, and Telex
      ingress remain closed.
    - [x] Explicit recursive material-content SO dispatch: the shared closed
      host-neutral envelope now has a fifth application input alternative.
      Structural input is selected by `replacementEvents` plus the presence of
      ordered `containerAssertions`; each assertion binds one container path
      and separate payload/result direct-item counts. The ASP adapter binds the
      named application to one `asp.v0` database and negotiates recursive
      inverse translation, the effective value-nesting policy, exact nested
      owner-set reconstruction, complete result and candidate validation,
      atomic revision, durable commit/lookup, and the recursive receipt journal.
      The depth limit is trusted adapter configuration, not transported AES
      data. Policy phases remain distinct, and direct durable and SO execution
      return identical receipt bytes. `telexIngress: false` keeps Telex,
      Wire/CLI, bare-event, generic AET, and AEON security-envelope ingress
      closed.
    - [x] Recursive child binding-owner identity and datatype metadata: every
      independently stored descendant binding may carry portable `identity`
      plus the complete `datatype`/`generics`/`clarifiers` triple. The inverse
      canonically combines that triple for ASP v0 storage, and strict portable
      reprojection must expand it to the same semantic components. The atomic
      application transports identity and the combined descriptor into each
      exact `put_assignment`; ordinary ASP candidate validation retains
      document-wide structural-identity uniqueness. Event-array ordering stays
      exact while JSON record-key insertion order is explicitly non-semantic.
      The durable and SO paths need no new envelope fields and reproduce the
      same metadata-bearing receipt; the adapter advertises exact recursive
      owner metadata. Existing root metadata remains target-owned, inline tuple
      items have no independent metadata owner, `origin`/`span` remain trusted
      host provenance rather than transported authority, and attribute trees
      fail with `AES_COMPAT_MATERIAL_ATTRIBUTES_UNSUPPORTED` pending their own
      inverse. Node-head storage and material owners inside tuples remain open.

</details>

The implementation record also closed the previously missing
`SansaAddressLiteral` ASP value-family boundary: JSON and compact AEON codecs,
indexed references, replay, snapshots, AEOS/SANSA reconstruction, and portable
export retain it as a distinct canonical value kind. Remaining work formerly
listed under broad ASP bullets is now classified in the primary checklist
above.

## 3. Compatibility and stored-data migration

- [x] Define a versioned compatibility projection from legacy
  `kind=node,value=tag` records to an outer `NodeLiteral` plus indexed
  `NodeHead`.
- [x] Define compatibility behavior for stored events created before stable
  structural identity was required: preserve a present identity and leave a
  missing one absent rather than manufacturing it.
- [x] Preserve legacy numeric node-child meaning; do not reinterpret it as a
  node-head path without an explicit versioned projection.
- [x] Keep durable history in its original contract by default and expose a
  versioned read-time compatibility view. An explicitly authorized migration
  writes a new versioned store after backup, replay, index, and restore proof.
- [x] Implement named, versioned legacy-to-portable adapters and conversion
  reports for every supported implementation-specific source contract.
  - [x] TypeScript: `aeon.typescript.assignment-events.v0-to-aes.events.v0`
    is exported by the AES package and Core facade, preserves source order and
    expanded paths, and returns a report alongside strict portable events.
  - [x] Rust: `aeon.rust.assignment-events.v0-to-aes.events.v0` is exported by
    `aeon-core` with the same report semantics and explicit document-projection
    option.
  - [x] Python: `aeon.python.assignment-events.v0-to-aes.events.v0` is exported
    by the package root with the same report semantics and explicit
    document-projection option.
  - [x] PHP: `aeon.php.assignment-events.v0-to-aes.events.v0` is exposed by
    `PortableEvents::adapt()` with the same logical report fields and explicit
    document-projection option.
  - [x] ASP: `asp.read-result.v0-to-aes.events.v0` derives a validated,
    reader-first `aes.complete.v0` body view from immutable ASP v0 history. Its
    report accounts for transformations, synthesis, omissions, semantic loss,
    and explicit loss authorization. Cache coordinates bind the trusted
    database/history identity plus exact source revision, target profile and
    projection, and adapter version; a separate fingerprint detects altered
    derived content. Portable writers remain disabled.
  - [x] AES-DB/ASP: expose the ASP adapter through a read-only current and
    historical view facade with a bounded derived-view cache. Revision changes
    cannot hit an older view, strict reads cannot reuse authorized-loss views,
    and explicit refresh, bypass, and scoped invalidation are available. Cache
    state is excluded from durable history and backup artifacts; full-log
    replay, ordinary checkpoint-bound restart, retained-checkpoint activation,
    exact full-log and checkpoint-suffix restore, and point-in-time restore
    reproduce the expected cache keys and content fingerprints from an initially
    empty cache. The retained history floor is enforced before cache lookup, so
    a cached pre-floor view cannot escape compaction policy.
  - [x] ASP Wire v0 and `aspcli`: expose explicit strict read-only portable AES
    views for current or selected historical database revisions. Wire requires
    a configured portable reader and permits only scope/revision parameters;
    the CLI does not offer semantic-loss authorization. Neither surface adds a
    portable transaction or writer route, and the wire method has a conformance
    fixture.
  - [x] ASP Telex export: derive canonical `telex.aes=0` only from a validated
    named strict read view, emit `aes.complete.v0` explicitly, and retain the
    compatibility view beside Telex on the library/Wire surfaces. `aspcli`
    supports raw current or historical Telex export. No surface accepts Telex
    as an actionable transaction or exposes semantic-loss authorization; Wire
    conformance covers the encoding.
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

- [x] Phase 1 — freeze and publish the transport-neutral `aes.events.v0`
  contract and its version discriminator.
- [x] Phase 2 — update `aeonite-specs` and land shared CTS vectors.
- [ ] After the canonical specification revision is committed, advance the
  `aeonite-website/specs.lock.json` revision and source digest, rebuild the
  publication artifacts, and deploy the published lifecycle metadata.
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
