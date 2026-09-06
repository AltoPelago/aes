# Portable AES Compatibility Contract v0

Scope: explicit legacy adapters, durable read views, and reader-before-writer
migration for portable AES.

Portable contract: `aes.events.v0`

This document defines how existing implementation-specific AES records may be
read alongside the portable event contract. It also defines the deployment
barrier that applies before portable writers are enabled on durable or shared
surfaces.

It does not declare every historical JSON event shape to be portable AES.

## 1. Compatibility boundary

Portable AES has an explicit event-contract identity: `aes.events.v0`.
`aes.complete.v0` and `aes.partial.v0` are profiles within that contract; they
are not substitutes for the contract identity.

A carrier establishes the contract before profile defaults apply. Telex does
so through `telex.aes=0`, which maps to `aes.events.v0`. Other encodings,
protocol envelopes, database schemas, and API versions MUST bind their records
to `aes.events.v0` explicitly.

An untagged JSON object or array is not identified as portable AES merely
because it contains fields such as `path`, `kind`, or `value`. In particular,
omitting `profile` from an untagged legacy surface does not select
`aes.complete.v0`. That default applies only after the carrier has established
the portable contract.

Implementations MUST NOT infer legacy-versus-portable interpretation from
record shape. Legacy `$.a[0]` and portable `$.a[0]` can name different
occurrences, so shape detection cannot be made safe by inspecting surrounding
records.

## 2. Named adapters

There is no single legacy AES contract. Each compatibility adapter declares:

- one implementation-owned and versioned source-contract identifier;
- its source profile or storage schema when one exists;
- `aes.events.v0` as its portable target contract;
- the selected portable profile and projection;
- its adapter version; and
- whether loss requiring caller authorization is permitted.

A legacy endpoint or store may supply its source-contract identity through
configuration or schema metadata. The identity need not occur in every stored
record, but it MUST be resolved before any record is interpreted.

Adapters return the converted stream and a conversion report. The report
identifies source and target contracts, adapter version, profile, projection,
and every omitted, synthesized, or unrepresentable field or occurrence. The
default mode succeeds only for a semantically lossless conversion under the
selected target context. An explicitly authorized lossy mode records each
loss; it never silently drops data.

## 3. Legacy to portable projection

### 3.1 Nodes and paths

A legacy record with `kind=node,value=tag` becomes:

1. a value-less `NodeLiteral` at the legacy node path; and
2. a `NodeHead` with the tag payload at index `[0]` below that node.

All legacy node descendants gain the head level. The adapter translates paths
using structural context, not textual replacement. For a node at `$.a`:

| Legacy occurrence | Portable occurrence |
| --- | --- |
| node | `$.a` |
| implicit tag carried by node | `$.a[0]` |
| first child `$.a[0]` | `$.a[0][0]` |
| second child `$.a[1]` | `$.a[0][1]` |

The same translation is applied recursively to descendant addresses and to
`CloneReference` and `PointerReference` payloads. A numeric legacy child is
never reinterpreted as a portable node head.

The adapter fails when it cannot prove from its declared source contract which
records are nodes or which reference domain a payload uses. It does not guess
from an isolated path.

### 3.2 Attributes

An embedded legacy attribute tree is emitted as ordinary portable events below
the owner's `.@` address space. Nested attributes and structural descendants
are flattened recursively in declared order. Attribute scope and lookup remain
downstream concerns.

If two distinct legacy occurrences—including an embedded attribute and an
already-flat record—map to the same portable address, the adapter reports
`AES_COMPAT_ADDRESS_COLLISION` and fails. Equal visible payloads do not make the
occurrences idempotent: identity, provenance, order, or assignment history may
still differ. A named source contract may designate one of two redundant views
as authoritative only when both views represent the same source occurrence;
that is schema mapping, not value-based deduplication.

If duplicates instead reach portable validation, `aes.complete.v0` reports
`AES_DUPLICATE_PATH`. Intentional repeated addresses remain distinct ordered
records under `aes.partial.v0`.

### 3.3 Identity

A legacy structural occurrence identity maps to `identity` on the portable
event representing the same occurrence. Node binding identity belongs on the
outer `NodeLiteral`; node-tag identity belongs on `NodeHead`; attribute-entry and
anonymous-child identities remain on their own events.

A missing legacy identity remains absent. Compatibility conversion never
manufactures identity from a path, value, hash, array position, or storage key.
Duplicate identities that violate the selected portable profile cause
validation failure.

### 3.4 Headers

Legacy synthetic `aeon:*` body events are excluded from the default body-only
projection. When `projection=aeon.document.v0` is explicitly selected, a
source-contract-specific adapter may move recognized header events to the
`header` address plane and MUST prove that no ordinary body binding is being
reclassified.

An adapter fails rather than infer header status from an `aeon:`-shaped key
when the legacy contract permits the same spelling as ordinary body data.

### 3.5 Provenance and spans

Legacy character, code-point, or UTF-16 offsets are not relabelled as portable
UTF-8 byte spans. When the adapter has the exact source bytes, it computes
`origin` from those bytes and may translate a verified source range to UTF-8
byte offsets. When it knows the source bytes but not an exact record range, it
emits `origin` alone. Otherwise it omits both `origin` and `span` and records
the provenance loss in the conversion report.

Origin and span are optional, so their absence does not by itself make a
semantic conversion lossy. It does make the conversion record- and
provenance-lossy.

### 3.6 Values, ordering, and derived fields

Legacy AST-shaped values are reduced to portable `kind`, canonical `value`, and
the declared `datatype`, `generics`, and `clarifiers` fields. A legacy combined
datatype descriptor is parsed once by the named adapter into the recursive
portable form; string and numeric clarifiers remain distinct and numeric
payloads remain strings. Conversion fails when the declared source contract
cannot establish those fields without implementation-specific guessing.

Derived keys, normalized paths, cached selectors, and nested AST objects are
not transported. Source order is retained unless the selected source contract
explicitly defines another authoritative order. Conversion never silently
sorts ledger or delivery streams.

## 4. Portable to legacy projection

Portable-to-legacy conversion is explicit and is not assumed to be lossless.
The adapter first proves that the complete selected stream is representable by
its named legacy target contract.

At minimum, a down-conversion adapter MUST reject:

- a node with zero or multiple heads when the target requires one
  (`AES_COMPAT_UNREPRESENTABLE`);
- a direct reference to a synthetic `NodeHead` occurrence;
- a partial stream whose missing context is needed to rebuild legacy nesting;
- identity, extension, header, or provenance data unsupported by the target,
  unless the caller explicitly authorizes and receives a report of that loss;
- duplicate or incompatible portable occurrences that collapse to one legacy
  address; and
- a portable value kind or payload with no exact target representation.

For a representable one-head node, the adapter collapses the `NodeHead` value
into the legacy node record and removes exactly one proven head index from each
content path and reference target. Flat attributes may be nested only after
complete ownership and order have been established.

## 5. Durable history

The default persistence strategy keeps existing legacy history immutable and
provides a versioned read-time projection into `aes.events.v0`. It does not
rewrite an existing log, snapshot, checkpoint, signature, digest, or span in
place.

The source contract, adapter version, target profile, and target projection are
part of read-view metadata. A cached projection is keyed by those values plus
the immutable source revision. Indexes derived from portable paths, node
heads, attributes, datatypes, identities, or references belong to that
versioned view and are rebuilt when any key component changes.

An operator may instead migrate into a new versioned store. Migration requires
a retained original or verified backup, deterministic replay, conversion
reports, rebuilt indexes, and restore verification before cutover. The migrated
store receives a new contract identity; it is never presented as the unchanged
legacy history.

## 6. Reader-first deployment barrier

Portable writers remain disabled while compatibility readers are introduced.
The rollout sequence is:

1. inventory every required reader, relay, validator, signer, indexer, and
   durable consumer on the target path;
2. deploy readers that require explicit contract context and either understand
   the selected portable profile and projection or reject them deterministically;
3. obtain acknowledgement from every required deployment and exercise mixed
   legacy/portable reads, conversion failures, replay, backup, and restore;
4. observe a healthy reader-canary window;
5. close or fence legacy writer routes for the selected canary cohort and
   verify that no unclassified writes reach the target;
6. enable portable writers for that explicit cohort only; and
7. expand writers only after a healthy, zero-loss canary and verified downstream
   delivery.

Before the first portable durable write, recovery is `legacy-safe`: disabling
the new reader view returns the system to legacy interpretation. After that
write, recovery is `compatible-reader-required`; rollback may disable portable
writers but MUST retain a reader capable of recognizing already-written
portable data.

Dual-writing legacy and portable records into the same unversioned namespace
is forbidden. Their paths, ordering, hashes, and signatures can differ even
when they represent the same source semantics.

## 7. Capability and evidence

A reader acknowledgement names `aes.events.v0`, each supported profile, and
each supported projection. Support for `aes.complete.v0` does not imply support
for `aes.partial.v0`, and body-only support does not imply
`aeon.document.v0` support.

Writer activation evidence records the exact contract, profile, projection,
adapter versions, reader deployments, target stores or interfaces, canary
cohort, monitoring window, and recovery rehearsal. A configuration flag or
successful unit test alone is not reader readiness.

Signatures and semantic hashes bind the contract identity, selected profile,
projection, ordering policy, and expanded datatype structure. A converted
stream is not verified under a legacy signature merely because its semantic
values match; signature translation requires a separately defined and
authorized signing operation.

## 8. Compatibility diagnostics

Compatibility adapters use stable codes; prose is not normative:

| Code | Condition |
| --- | --- |
| `AES_COMPAT_CONTEXT_REQUIRED` | source or target contract context is absent |
| `AES_COMPAT_UNSUPPORTED_CONTRACT` | named source or target contract is unsupported |
| `AES_COMPAT_AMBIGUOUS_PATH` | structural context cannot establish path meaning |
| `AES_COMPAT_ADDRESS_COLLISION` | distinct source occurrences map to one target address |
| `AES_COMPAT_UNREPRESENTABLE` | a source occurrence has no exact target representation, including zero or multiple heads for a single-head target |
| `AES_COMPAT_LOSS_REQUIRES_OPT_IN` | conversion would discard data without authorization |
| `AES_COMPAT_SOURCE_REQUIRED` | exact source bytes are required for requested provenance conversion |
| `AES_COMPAT_VIEW_INVALID` | a serialized compatibility view or its bound cache/integrity metadata is invalid |
| `AES_COMPAT_READER_NOT_READY` | a required reader has not acknowledged the selected context |
| `AES_COMPAT_WRITER_DISABLED` | writer activation evidence is incomplete or unhealthy |

These diagnostics belong to the adapter and rollout layer. Once conversion
succeeds, the resulting stream is validated independently under the portable
AES diagnostic rules.

## 9. Conformance requirements

Shared compatibility vectors cover at least:

- legacy node-child paths and nested-node reference targets;
- embedded and nested attributes, including address collisions;
- present, absent, and duplicate structural identities;
- body data that resembles a legacy synthetic header;
- ASCII, multibyte, combining, and astral source ranges;
- unsupported value kinds and direct `NodeHead` references;
- zero- and multiple-head down-conversion into single-head targets;
- partial-stream down-conversion without required ancestry;
- rejection of untagged input and unknown contract identifiers;
- conversion reports for every authorized loss; and
- mixed-version read, writer-canary, rollback, replay, backup, and restore.

An implementation claims compatibility only for the named source contracts,
profiles, and projections exercised by those vectors.
