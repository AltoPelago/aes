# Current AES event-model inventory

Status: working note

Surveyed: 2026-09-04

This note records the starting point for `telex.aes`. It is descriptive, not a
new source of truth.

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

Draft 0 does not transport source lexemes. Exact token spelling remains in the
source and may be recovered through provenance; portable AES retains only the
recognized value-kind, datatype, and payload distinctions.

The complete canonical datatype descriptor is carried in one `datatype` field.
Generic arguments and clarifiers stay on the declaring event and are not
inferred onto descendants. Clone and pointer references use distinct kinds with
the same canonical target-path payload shape. Trimtick processing occurs before
AES emission, so its decoded and trimmed result is transported as an ordinary
`string`; delimiter width and indentation remain source concerns. World time
context is retained as the distinct `wtc` kind.

Anonymous typed values are flattened into the event at their indexed path. The
anonymous head's datatype and structural identity become the same `datatype`
and `identity` fields used by named bindings; there is no `typed-value` wrapper
kind. Node values are projected one structural level further: the outer `node`
event is a value-less ordered container and each `node-head` descendant carries
its tag. Current AEON produces one head at index zero, while the AES shape does
not preclude future empty or multi-headed node profiles.

## Recommended boundary

The working recommendation is:

```text
implementation event / AST
          |
          | explicit adapter
          v
portable AES event profile
          |
          +---- telex.aes
          |
          +---- film.aes
```

The adapter is important. It makes implementation-only data loss or derivation
visible and testable instead of burying it inside a serializer.

The Draft 0 direction deliberately makes the portable side flat:

```text
path + kind + optional datatype/value/identity/provenance
path + kind + optional datatype/value/identity/provenance
path + kind + optional datatype/value/identity/provenance
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
