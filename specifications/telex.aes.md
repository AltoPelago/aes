# `telex.aes` Draft 0

Status: exploratory draft

Format version: `0`

File suffix: `.telex.aes`

## 1. Purpose

`telex.aes` is the textual interchange format for an ordered Assignment Event
Stream.

Its job is intentionally narrow:

> Encode already-recognized AES events so another component can reconstruct
> them without parsing AEON source, applying type inference, or depending on a
> producer's in-memory object shape.

Telex and the future `film.aes` encode the same portable AES event model. Telex
optimizes for trivial implementation, inspection, fixtures, diffs, and protocol
debugging. Film will optimize for binary transport and size.

This draft is a candidate to discuss and test. The version number `0` is not a
compatibility promise.

## 2. Design constraints

Telex should be:

- UTF-8 text;
- streamable one event at a time;
- expressible as flat `field=value` lines;
- deterministic in canonical form;
- independent of JSON and host-language data models;
- explicit about value kind and representation;
- able to carry all information in the portable AES event contract; and
- small enough that a syntax implementation can be audited end to end.

Compactness is secondary. An AEON document may be smaller because its syntax
allows context to be inferred. Telex makes that context explicit.

## 3. Example

Given this illustrative AEON input:

```aeon
customer:object = {
  name:string = "Alice"
  balance:decimal = %010.00
}
```

A Telex projection could be:

```text
telex.aes=0

path=$.customer
kind=object
datatype=object
span=1:1:0-4:2:73

path=$.customer.name
kind=string
datatype=string
value=Alice
representation=quoted
span=2:3:22-2:24:43

path=$.customer.balance
kind=radix
datatype=decimal
value=010.00
span=3:3:46-3:28:71
```

The object event carries its own representation kind. Its children are ordinary
subsequent events. `value=010.00` is a textual payload whose meaning is fixed by
`kind=radix` and `datatype=decimal`; it is not parsed as a Telex number.

## 4. Physical format

### 4.1 Encoding

A Telex file is UTF-8 without a byte-order mark.

Canonical Telex uses LF (`U+000A`) line endings and ends with one LF. A decoder
MAY accept CRLF and normalize it before parsing. It MUST reject a bare CR and
invalid UTF-8.

Telex performs no Unicode normalization. A conforming implementation preserves
the decoded Unicode scalar sequence exactly.

### 4.2 Preamble

The first line is exactly:

```text
telex.aes=0
```

The preamble is followed by a blank line when at least one event follows. Future
format versions use a different value, not an inferred feature set.

### 4.3 Event framing

An event is a non-empty stanza of `field=value` lines. One or more blank lines
separate stanzas. A canonical encoder emits exactly one blank line between
events.

The first `=` on a line separates the field name from its payload. Later `=`
characters belong to the payload and do not need escaping.

Field names use lowercase ASCII letters, digits, `-`, and `.`. A field and each
dotted field segment begin with a letter. Empty payloads are valid. Empty field
names, duplicate fields in one event, and lines without `=` are invalid.

There are no comments. Comments would create a second information channel and
would complicate byte-for-byte canonicalization.

### 4.4 Payload escaping

Payloads are single logical lines. The Telex escape vocabulary is:

| Escape | Decoded scalar |
| --- | --- |
| `\\\\` | backslash |
| `\\n` | line feed |
| `\\r` | carriage return |
| `\\t` | horizontal tab |
| `\\0` | null |
| `\\u{H...}` | Unicode scalar written as 1-6 uppercase hexadecimal digits |

An unescaped C0 control or DEL is invalid. A surrogate, a value above
`U+10FFFF`, an unknown escape, and a lowercase or padded canonical Unicode
escape are invalid in canonical input.

Canonical encoders use the short escapes where available, escape other C0
controls and DEL with `\\u{...}`, and emit all other Unicode scalars directly.

Escaping is defined by Telex. It is not JSON, JavaScript, Rust, or AEON source
escaping.

## 5. Portable event profile

The following is the Draft 0 candidate event profile. It needs reconciliation
with the AES specification and the implementations before promotion.

### 5.1 Core fields

| Field | Presence | Meaning |
| --- | --- | --- |
| `path` | required | canonical SANSA data address of the binding |
| `kind` | required | portable representation kind token |
| `span` | required for source events | original source range |
| `datatype` | optional | declared datatype, without inference |
| `identity` | optional | structural identity carried by the binding |
| `value` | kind-dependent | decoded textual payload |
| `representation` | kind-dependent | representation detail not captured by `kind` |
| `lexeme` | optional | opaque original source spelling for source-fidelity profiles |

Every stanza after the preamble is an AES event. Draft 0 therefore has no
`event=assignment` discriminator. Adding the same constant to every stanza
would not carry information.

`key` is not transported because it is derivable from the final segment of
`path`. A normalized selector path is also derived data and is not transported.

The stream order is the AES event order. An encoder MUST NOT sort events by
path.

### 5.2 Spans

The candidate span form is:

```text
start-line:start-column:start-byte-end-line:end-column:end-byte
```

Lines and columns are one-based. Columns count Unicode scalar values. Byte
offsets are zero-based UTF-8 offsets. The end position is exclusive.

This deliberately specifies both human-facing positions and stable byte
offsets. Existing implementations currently use host-specific position shapes;
the exact portable span contract is a decision gate for Draft 0.

Events created without source text need an explicit provenance rule. They must
not invent a zero span and imply source evidence that does not exist. The likely
direction is an optional `origin` field plus omission of `span`, but that is not
fixed in this draft.

### 5.3 Representation kinds

The initial vocabulary to reconcile is:

```text
string
number
infinity
nan
null
boolean
toggle
hex
radix
encoding
separator
sansa-address
date
datetime
time
object
list
tuple
node
clone-reference
pointer-reference
```

Container events carry their kind but no nested value tree. Every member or
item follows as an event at its own descendant canonical path. The same rule
applies recursively to object members, list and tuple elements, node children,
and values reached through attribute address spaces.

A node additionally needs `node.tag` and may need `node.datatype`. References
need a canonical `target`. Null reasons and other kind-specific information need
fixed fields rather than inference from the payload.

The exact per-kind field table belongs to the portable AES value-representation
contract, not to the generic line parser. It is the largest unfinished part of
this draft.

### 5.4 Flat structure and attributes

AES has one flat event model. It does not embed attribute collections inside an
owning event and Telex does not have an attribute-specific payload grammar.

For example:

```aeon
a@{a={a=1, b@{a=2}=2}}=0
```

is represented by events at:

```text
$.a
$.a.@.a
$.a.@.a.a
$.a.@.a.b
$.a.@.a.b.@.a
```

Each event uses the same `path`, `kind`, datatype, identity, value,
representation, lexeme, and provenance fields as any other event.

The `.@` segment is part of the canonical path. Telex preserves it but does not
interpret attribute ownership or scope. SANSA and downstream consumers own
address-space interpretation, just as consumers interpret object members,
indexed elements, and node children.

Telex preserves event order exactly. It does not derive a new order from path
structure and it never groups, sorts, or nests attribute-space events.

## 6. Canonical form

A canonical Telex encoder emits:

1. the exact version preamble;
2. one blank line before the first event;
3. events in their semantic stream order;
4. fields in the order below;
5. extension fields in Unicode-code-point order after core fields;
6. the shortest canonical escape for each payload scalar;
7. one blank line between events; and
8. exactly one final LF.

Core field order:

```text
path
kind
datatype
identity
value
representation
lexeme
span
```

A tolerant decoder may accept non-canonical field order, multiple stanza
separators, CRLF, and lowercase hexadecimal in Unicode escapes. It must expose
that the input was non-canonical when canonical bytes matter.

## 7. Validation layers

Telex deliberately separates three checks:

1. **Syntax:** UTF-8, framing, field grammar, duplicates, and escapes.
2. **Event shape:** required and allowed fields for each event and value kind.
3. **Stream semantics:** canonical paths, uniqueness, ordering, structural
   consistency, datatypes, references, and profile rules.

A tiny Telex parser may implement only layer 1. It must not claim AES
conformance merely because it can split fields.

## 8. Unknown fields and versions

Draft 0 syntax parsers preserve unknown fields. Semantic decoders reject an
unknown required core field or unsupported event kind unless an explicitly
negotiated profile defines it.

Silently discarding unknown data would make round trips lossy. Treating an
unknown field as harmless without profile knowledge could change meaning.

Version negotiation belongs to an enclosing protocol such as `poem.aes`.
Opening a standalone file with an unsupported version fails explicitly.

## 9. Security and resource bounds

Decoders must accept caller-supplied limits for at least:

- input bytes;
- line bytes;
- fields per event;
- event count;
- decoded payload bytes; and
- canonical path depth.

Syntax decoding performs no reference resolution, schema loading, network
access, datatype execution, or source-language evaluation.

## 10. Decision gates before Draft 1

The following must be settled with fixtures and at least two implementations:

1. Is `lexeme` part of core AES, an optional source-fidelity profile, or omitted?
2. What are the exact fields and canonical payloads for every value kind?
3. What is the portable source-span coordinate system?
4. How are anonymous typed values and structural identities represented on
   their flat events?
5. Which producer-side ordering constraints apply beyond Telex's requirement to
   preserve the supplied event order exactly?
6. Does a transport stream require prefix completeness, or may a declared raw
   event profile carry orphaned paths?
7. Which unknown-field behavior is safe for standalone files and negotiated
   protocols?
8. What media type and profile identifiers are registered for Telex?

Draft 1 should not be declared until the answers exist as conformance vectors,
not only prose.

## 11. Relationship to the Aeonic Semantic Language

Telex records representation without applying higher-order value behavior.

For example, Telex can carry that a payload has kind `radix`, datatype
`decimal`, and value `010.00`. The Aeonic Semantic Language owns questions such
as equality, ordering, conversion, precision, and arithmetic for that value.

This keeps the layers distinct:

```text
Telex / Film
    carry recognized values

Aeonic Semantic Language
    defines shared operations over values

AEOS / SANSA / Tonics / storage
    apply those operations in their own domains
```
