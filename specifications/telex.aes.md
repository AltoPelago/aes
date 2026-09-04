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
- explicit about value kind and declared datatype;
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
| `kind` | required | portable value-kind token |
| `datatype` | optional | declared datatype, without inference |
| `identity` | optional | structural identity carried by the binding |
| `value` | kind-dependent | decoded textual payload |
| `span` | required for source events | original source range |

Every stanza after the preamble is an AES event. Draft 0 therefore has no
`event=assignment` discriminator. Adding the same constant to every stanza
would not carry information.

`key` is not transported because it is derivable from the final segment of
`path`. A normalized selector path is also derived data and is not transported.

The stream order is the AES event order. An encoder MUST NOT sort events by
path.

### 5.2 Datatypes

`datatype` carries the complete canonical datatype descriptor as one textual
payload. It is not limited to a bare type name.

For example:

```text
datatype=csv["."]
datatype=list<int>
datatype=null<string>
```

The AEON binding `:` is not part of the descriptor. Generic arguments and
clarifiers are retained in their canonical form because they are declared
semantic claims, not source formatting.

A datatype remains only on the event where it was declared. AES does not infer
or propagate `int` from `list<int>` onto the list's child events. Interpreting a
container generic, validating it against descendants, or assigning inherited
meaning is downstream work.

Telex treats the descriptor as a payload. Its grammar and canonical rendering
belong to the portable AES/Aeonic type contract rather than the Telex framing
grammar.

### 5.3 Spans

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

### 5.4 Value kinds

The portable value-kind vocabulary and its `value` payloads are:

| `kind` | `value` payload |
| --- | --- |
| `string` | decoded Unicode string; an empty payload is valid |
| `number` | canonical finite numeric text |
| `infinity` | `Infinity` or `-Infinity` |
| `nan` | `NaN` or `-NaN` |
| `null` | recognized sentinel or decoded custom reason |
| `boolean` | `true` or `false` |
| `toggle` | `yes`, `no`, `on`, or `off` |
| `hex` | lowercase hexadecimal digits, without `#` or visual underscores |
| `radix` | canonical radix payload, without `%` or visual underscores |
| `encoding` | encoding payload without `&`; padding is preserved |
| `separator` | canonical separator payload without `^` |
| `sansa-address` | canonical SANSA address |
| `date` | canonical date text |
| `time` | canonical time text |
| `datetime` | canonical date-time text |
| `wtc` | canonical world-time-context text |
| `object` | absent |
| `list` | absent |
| `tuple` | absent |
| `node` | absent |
| `node-head` | node tag |
| `clone-reference` | canonical target path |
| `pointer-reference` | canonical target path |

`value` is required for every kind except `object`, `list`, `tuple`, and
`node`, for which it is absent. Payloads are strings at the Telex layer even
when their kind gives them numeric, temporal, or other semantics.

AEON source spelling is normalized before the event enters AES. In particular,
quoted strings, backtick strings, and trimtick strings all use `kind=string`.
A trimtick payload has already undergone its trimming operation; delimiter
width and indentation are source mechanics rather than properties of the
resulting value. Numeric separators and the leading AEON sigils for hex,
radix, encoding, and separator values are likewise not transported.

The `separator` payload remains an opaque canonical value in AES. Its declared
datatype and clarifiers may assign structure or interpretation downstream;
Telex does not split it into a nested payload.

Container events carry their kind but no nested value tree. Every member or
item follows as an event at its own descendant canonical path. The same rule
applies recursively to object members, list and tuple elements, node children,
and values reached through attribute address spaces.

A node head uses `value` for its tag. A reference uses `value` for its canonical
target path. Nulls use `value` for their recognized sentinel or decoded custom
reason. No kind introduces a kind-specific field name.

Clone and pointer references remain distinct kinds even when they target the
same canonical path:

```text
path=$.copy
kind=clone-reference
value=$.source

path=$.alias
kind=pointer-reference
value=$.source
```

The AEON `~` and `~>` sigils are not retained in `value`; their distinction is
carried by `kind`. A Telex decoder preserves the symbolic reference and does not
resolve or materialize it.

The table defines the portable AES value representation. A generic Telex syntax
parser need not validate it, but an AES event-shape validator must.

### 5.5 Anonymous heads and structural identity

An anonymous typed or attributed value does not introduce a wrapper event or a
special value kind. Its datatype, structural identity, and value kind belong to
the event at its indexed path.

For example:

```aeon
values:list = [
  \item-1\@{source:string = "user"}:int = 3
]
```

projects as:

```text
path=$.values
kind=list
datatype=list

path=$.values[0]
kind=number
datatype=int
identity=item-1
value=3

path=$.values[0].@.source
kind=string
datatype=string
value=user
```

The `identity` payload omits the AEON backslash delimiters. It is occurrence
metadata and does not alter the event path or stream order. Telex does not
invent an identity when none was supplied. Document-level uniqueness is an AES
semantic constraint rather than Telex framing syntax.

### 5.6 Nodes

A node is an ordered structural container. Its node heads are ordinary flat
descendant events, addressed by index. A `node-head` carries the tag in `value`
and owns that head's ordered content values.

Current AEON syntax produces one head at index zero:

```aeon
a:node = <tag("hello", 2)>
```

```text
path=$.a
kind=node
datatype=node

path=$.a[0]
kind=node-head
value=tag

path=$.a[0][0]
kind=string
value=hello

path=$.a[0][1]
kind=number
value=2
```

Binding-head metadata remains on the `node` event. Metadata written on the AEON
node head belongs to its `node-head` event, so both heads remain independently
representable without a new namespace:

```text
$.a.@.x
$.a[0].@.x
```

Telex does not impose a cardinality of exactly one node head. The flat model can
represent an empty node or multiple ordered heads without changing its path or
event grammar. Whether a source language or negotiated AES profile permits
zero, one, or many heads is a semantic constraint. This preserves room for
future forms equivalent to `<>` and `<<tag>, <tag>>` while current AEON remains
single-headed.

Nested nodes follow directly from the same rule. For
`a = <tag(<tag>, <tag>)>`, the inner node containers occur at `$.a[0][0]` and
`$.a[0][1]`; their heads occur at `$.a[0][0][0]` and `$.a[0][1][0]`.

### 5.7 Flat structure and attributes

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

Each event uses the same `path`, `kind`, datatype, identity, value, and
provenance fields as any other event.

The `.@` segment is part of the canonical path. Telex preserves it but does not
interpret attribute ownership or scope. SANSA and downstream consumers own
address-space interpretation, just as consumers interpret object members,
indexed elements, and node children.

Telex preserves event order exactly. It does not derive a new order from path
structure and it never groups, sorts, or nests attribute-space events.

### 5.8 Source lexemes

AES does not carry the exact original source token or a `lexeme` field.

`kind`, `datatype`, and `value` preserve the recognized value distinctions.
Quote choice, escape spelling, numeric separators, and other source-authoring
details remain in the original source. Source-aware tooling may recover them
through provenance and spans.

This boundary prevents canonical Telex bytes from changing merely because two
sources use different spellings for the same portable AES value.

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

1. What is the portable source-span coordinate system?
2. Which producer-side ordering constraints apply beyond Telex's requirement to
   preserve the supplied event order exactly?
3. Does a transport stream require prefix completeness, or may a declared raw
   event profile carry orphaned paths?
4. Which unknown-field behavior is safe for standalone files and negotiated
   protocols?
5. What media type and profile identifiers are registered for Telex?

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
