# Current AES event-model inventory

Status: working note

Surveyed: 2026-09-04

This note records the starting point for `telex.aes`. It is descriptive; the
transport-neutral source of truth is
[`../specifications/aes.events.md`](../specifications/aes.events.md).

## Shared core

The normative AES appendix and the current TypeScript, Rust, and Python
implementations broadly agree on an ordered assignment event with:

- a canonical path;
- an original or representation-preserving value;
- source provenance;
- an optional declared datatype; and
- optional binding attributes.

They also agree that comments belong to a parallel annotation stream, not AES.

## Current differences

| Concern | Specification | TypeScript | Rust | Python JSON surface |
| --- | --- | --- | --- | --- |
| Path | `CanonicalPath` | structured segments | structured segments | formatted string |
| Local key | not listed | `key` | `key` | `key` |
| Normalized selector | not listed | optional `normalizedPath` | absent | absent |
| Structural identity | not in current appendix | optional `structuralId` | absent on event | absent |
| Attributes name | `attributes` | `annotations` map | `annotations` map | optional `annotations` object |
| Value | `ASTValue` | parser `Value` object | `Value` enum | JSON object |
| Span | `SourceSpan` | lexer `Span` | start/end positions | JSON projection |
| Nested containers | not fully pinned | parent value plus emitted descendants | full or shallow option | nested JSON plus descendants |

The CLI JSON shapes are debugging and integration surfaces. They are not a
stable interchange contract and they do not expose every in-memory field in the
same way.

## Consequences for Telex

Serializing one current JSON object shape would standardize accidental host
details and still fail to round-trip another implementation.

The portable profile needs explicit decisions for:

1. **Path:** transport the canonical string and use SANSA as its grammar owner.
2. **Derived fields:** omit `key` and normalized selectors because consumers can
   derive them from the canonical path and active selector rules.
3. **Structure:** use shallow value events plus explicit descendant events;
   never duplicate a nested value tree inside a parent event.
4. **Identity:** decide whether structural identity is now part of the AES event
   contract and add it consistently if so.
5. **Attributes:** emit their values as ordinary flat events at canonical paths
   containing `.@`; do not carry an embedded attribute collection.
6. **Values:** distinguish portable representation data from implementation AST
   details such as class names, maps, enum variants, and cached parsed forms.
7. **Provenance:** define span units and the behavior of events that were not
   produced directly from source text.
8. **Ordering:** reconcile lexical binding order with synthetic indexed child
   events and future database-originated events.

Draft 0 makes three fidelity boundaries explicit. Telex record round trips
preserve ordered field payloads, including provenance and unknown fields, but
canonicalization need not preserve tolerant input bytes. Portable AES semantic
round trips preserve the selected profile's addresses, kinds, canonical
values, datatypes, identities, significant extensions, and authoritative
order. Provenance is separately preservation-sensitive but is not semantic
equality. Exact AEON source reconstruction is outside the guarantee: origin
and span can locate a separately retained matching artifact but cannot recreate
discarded lexemes, whitespace, comments, header spelling, BOM, or line endings.

The complete canonical datatype descriptor is carried in one `datatype` field.
Generic arguments and clarifiers stay on the declaring event and are not
inferred onto descendants. Clone and pointer references use distinct kinds with
the same canonical target-path payload shape. Trimtick processing occurs before
AES emission, so its decoded and trimmed result is transported as an ordinary
`string`; delimiter width and indentation remain source concerns. World time
context is retained as the distinct `wtc` kind. Its complete authored payload
preserves civil, explicit-offset, and UTC anchor forms independently across
lowercase `local`, named, and geographic references. AES does not resolve or
rewrite those forms. `conflictAuthority` remains trusted consumer policy, not
document authority or an AES event field.

Anonymous typed values are flattened into the event at their indexed path. The
anonymous head's datatype and structural identity become the same `datatype`
and `identity` fields used by named bindings; there is no `typed-value` wrapper
kind. Node values are projected one structural level further: the outer `node`
event is a value-less ordered container and each `node-head` descendant carries
its tag. Current AEON produces one head at index zero, while the AES shape does
not preclude future empty or multi-headed node profiles.

AEON source paths and portable AES event paths are distinct domains. Ordinary
segments remain stable, but crossing from a node to source child `[i]` expands
to node-head/content segments `[0][i]` in AES, recursively for nested nodes.
Reference payloads use the AES event-path domain and are translated with
document structure; structural identity never participates. A synthetic
node-head target has no current AEON reference spelling, so reverse projection
fails explicitly instead of reinterpreting it as the first child.

A source-backed `node-head` span begins at its tag token and ends after its last
identity, attribute, or datatype component. It excludes `<`, children, and
closing delimiters. Implementations that only retain the whole node-literal
span carry `origin` alone for the head until they can supply that exact range.

Portable spans use only an inclusive start and exclusive end measured as
zero-based UTF-8 byte offsets. Line and column coordinates are derived rather
than transported. Both provenance fields are optional. A source-less record
omits both; a known source may carry `origin` alone; and `span` is valid only
with `origin`. Draft 0 identifies the exact, unnormalized source bytes with a
record-local `sha256:<64 lowercase hex>` digest, including any accepted source
BOM and original line endings. This keeps multi-source streams flat and lets
Film compress repeated origins later.

Event order remains producer-profile dependent and is always preserved by
Telex. Canonical AEON document projection uses depth-first preorder, while a
ledger retains its original sequence. Signature profiles must explicitly state
whether they cover a canonical semantic projection or the exact supplied event
order.

The portable AES contract uses `x.<owner>.<name>` as the extension naming
convention. Telex preserves unknown fields, while semantic decoders fail closed
unless an active profile registers the exact field. The prefix never makes an
extension implicitly optional, and new required core fields require a new AES
event-contract version.

The complete, self-contained `aes.complete.v0` profile is the AES default.
Omitting the Telex profile declaration carries that default; streams that relax
cross-event constraints must explicitly declare `aes.partial.v0` or a future
specialized profile.
`aeon.gp.profile.v1` references `aes.complete.v0` explicitly so its projection
contract remains visible even though Telex would apply the same default.
The partial profile still requires individually valid AES events; it relaxes
cross-event ancestry, uniqueness, compatibility, and ordering claims rather
than turning arbitrary Telex stanzas into AES events.

AEON headers use an independent projection axis. The default AEON-to-AES
projection emits body events only. `projection=aeon.document.v0` explicitly
adds a complete, flat control plane whose records use
`header=$.["aeon:..."]` instead of `path`. Structured and shorthand AEON
headers normalize to the same records; those records precede body events and
never share the body address space. `profile` still selects AES completeness,
while `projection` selects what source document surface is represented.

TypeScript and PHP currently synthesize `aeon:*` AES events. Rust and Python
already retain header metadata separately while excluding it from public body
events. All four adapters need to converge on body-only output by default and
explicit `header` records only for the document projection.

## Recommended boundary

The working recommendation is:

```text
implementation event / AST
          |
          | explicit adapter
          v
portable AES event contract
          |
          +---- telex.aes
          |
          +---- film.aes
```

The adapter is important. It makes implementation-only data loss or derivation
visible and testable instead of burying it inside a serializer.

The Draft 0 direction deliberately makes each portable plane flat:

```text
path + kind + optional datatype/value/identity/provenance
path + kind + optional datatype/value/identity/provenance
path + kind + optional datatype/value/identity/provenance
```

An explicit AEON document projection prepends records of the parallel form:

```text
header + kind + optional datatype/value/identity/provenance
```

Path structure may identify an object member, indexed item, node child, or
attribute address space. AES does not give those cases different event shapes.

## Source material

- `aeonite-specs/sources/appendices/v1/appendix-aes.aeon`
- `aeon/implementations/typescript/packages/aes/src/events.ts`
- `aeon/implementations/typescript/packages/parser/src/ast.ts`
- `aeon/implementations/rust/crates/aeon-core/src/lib.rs`
- `aeon/implementations/rust/crates/aeon-core/src/flatten.rs`
- `aeon/implementations/python/src/aeon/core.py`
- `aeon/implementations/python/src/aeon/ast.py`
- `overview/proposals/0000-future-ideas.md`, section 6
- `overview/proposals/0011-naming.md`
- `overview/proposals/0016-structurally-navigable-aes.md`
